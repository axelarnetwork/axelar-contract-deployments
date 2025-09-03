// contracts/GLDToken.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import '@openzeppelin/contracts/token/ERC20/ERC20.sol';
import { Ownable } from '@openzeppelin/contracts/access/Ownable.sol';
import { AxelarExecutable } from '@axelar-network/axelar-gmp-sdk-solidity/contracts/executable/AxelarExecutable.sol';
import { IAxelarGasService } from '@axelar-network/axelar-gmp-sdk-solidity/contracts/interfaces/IAxelarGasService.sol';
import 'solidity-bytes-utils/contracts/BytesLib.sol';
import '@openzeppelin/contracts/utils/Strings.sol';

contract CrossChainBurn is Ownable, ERC20, AxelarExecutable {
    using BytesLib for bytes;
    using Strings for string;

    IAxelarGasService public immutable gasService;

    struct DestinationChainAndAddress {
        string destinationChain;
        string destinationAddress;
    }

    string public homeChain;

    mapping(string => bool) public crossChainAdmins;

    constructor(
        string memory name,
        string memory symbol,
        address admin_,
        string memory homeChain_,
        address gateway_,
        address gasService_
    ) ERC20(name, symbol) Ownable(admin_) AxelarExecutable(gateway_) {
        homeChain = homeChain_;
        gasService = IAxelarGasService(gasService_);
    }

    event TokenBurnedCrossChain(address indexed account, string sourceChain, string sourceAddress, address token, uint256 amount);

    function burn(address account, uint256 amount) external /*onlyOwner*/ {
        _burn(account, amount);
    }

    // Mint function - only owner can mint
    function mint(address to, uint256 amount) external /*onlyOwner*/ {
        _mint(to, amount);
    }

    function burnFromCrossChain(
        bytes calldata account,
        uint256 amount,
        DestinationChainAndAddress[] calldata destinationChainsAndAddresses
    ) external payable /*onlyOwner*/ {
        for (uint i = 0; i < destinationChainsAndAddresses.length; i++) {
            string memory destinationChain = destinationChainsAndAddresses[i].destinationChain;
            string memory destinationAddress = destinationChainsAndAddresses[i].destinationAddress;

            // Burn from other chain
            // Use encodePacked so it is easier to decode on another chain without abi support
            bytes memory payload = abi.encodePacked(amount, account);

            gasService.payNativeGasForContractCall{ value: msg.value }(
                address(this),
                destinationChain,
                destinationAddress,
                payload,
                msg.sender
            );
            gateway().callContract(destinationChain, destinationAddress, payload);
        }
    }

    // Functions
    function setCrossChainAdmin(string calldata sourceAddress, bool isAllowed) external onlyOwner {
        crossChainAdmins[sourceAddress] = isAllowed;
    }

    function setHomeChain(string calldata newHomeChain) external onlyOwner {
        homeChain = newHomeChain;
    }

    function _execute(bytes32, string calldata sourceChain, string calldata sourceAddress, bytes calldata payload) internal override {
        // require(_stringsEqual(sourceChain, homeChain));
        // Allow owner or cross chain admin to burn for another account
        // TODO uncomment
        // require(_stringsEqual(sourceAddress, Strings.toHexString(uint256(uint160(owner())), 20)) || crossChainAdmins[sourceAddress]);

        // Decodes the encodePacked encoded payload, which should be easy to create from other chains without abi support
        uint256 amount = payload.toUint256(0);
        address account = payload.toAddress(32);

        _burn(account, amount);

        emit TokenBurnedCrossChain(account, sourceChain, sourceAddress, address(this), amount);
    }

    /**
     * @dev Returns true if the two strings are equal.
     */
    function _stringsEqual(string memory a, string memory b) internal pure returns (bool) {
        return bytes(a).length == bytes(b).length && keccak256(bytes(a)) == keccak256(bytes(b));
    }

    function _addressToString(address _addr) internal pure returns (string memory) {
        bytes32 value = bytes32(uint256(uint160(_addr))); // Convert address to bytes32
        bytes memory alphabet = '0123456789abcdef'; // Hexadecimal alphabet

        bytes memory result = new bytes(42); // 40 characters + '0x'
        result[0] = '0';
        result[1] = 'x';

        for (uint i = 0; i < 20; i++) {
            result[2 + i * 2] = alphabet[uint8(value[i + 12] >> 4)]; // High nibble
            result[3 + i * 2] = alphabet[uint8(value[i + 12] & 0x0f)]; // Low nibble
        }

        return string(result);
    }
}
