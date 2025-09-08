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

    function burn(address account, uint256 amount) external {
        _burn(account, amount);
    }

    // Mint function - only owner can mint
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    function burnFromCrossChain(
        bytes calldata account,
        uint256 amount,
        string calldata destinationChain,
        string calldata destinationAddress
    ) external payable onlyOwner {
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

    // Functions
    function setCrossChainAdmin(string calldata sourceAddress, bool isAllowed) external onlyOwner {
        crossChainAdmins[sourceAddress] = isAllowed;
    }

    function setHomeChain(string calldata newHomeChain) external onlyOwner {
        homeChain = newHomeChain;
    }

    function _execute(bytes32, string calldata sourceChain, string calldata sourceAddress, bytes calldata payload) internal override {
        // TODO uncomment
        // require(_stringsEqual(sourceChain, homeChain));

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
}
