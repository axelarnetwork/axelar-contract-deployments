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
    mapping(address => bool) public frozenAccounts;

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
    event AccountFrozen(address indexed account, bool frozen);

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

        //TODO: uncomment once we have memento chainId
        // require(_getChainID() == mementoChainId, 'Cross-chain burn can only be performed on Memento');

        bytes memory payload = abi.encodePacked(amount, account, 1);

        gasService.payNativeGasForContractCall{ value: msg.value }(
            address(this),
            destinationChain,
            destinationAddress,
            payload,
            msg.sender
        );
        gateway().callContract(destinationChain, destinationAddress, payload);
    }


    function freezeAccountCrossChain(
        bytes calldata account,
        string calldata destinationChain,
        string calldata destinationAddress,
    ) external payable onlyOwner {

        //TODO: uncomment once we have memento chainId
        // require(_getChainID() == mementoChainId, 'Cross-chain burn can only be performed on Memento');

        bytes memory payload = abi.encodePacked(account, 2);

        gasService.payNativeGasForContractCall{ value: msg.value }(
            address(this),
            destinationChain,
            destinationAddress,
            payload,
            msg.sender
        );
        gateway().callContract(destinationChain, destinationAddress, payload);
    }

    // Freeze/Unfreeze functions
    function freezeAccount(address account) external onlyOwner {
        frozenAccounts[account] = true;
        emit AccountFrozen(account, true);
    }

    function unfreezeAccount(address account) external onlyOwner {
        frozenAccounts[account] = false;
        emit AccountFrozen(account, false);
    }

    function isAccountFrozen(address account) external view returns (bool) {
        return frozenAccounts[account];
    }

    // Override transfer functions to check for frozen accounts
    function transfer(address to, uint256 amount) public override returns (bool) {
        require(!frozenAccounts[msg.sender], 'Account is frozen');
        require(!frozenAccounts[to], 'Recipient account is frozen');
        return super.transfer(to, amount);
    }

    function transferFrom(address from, address to, uint256 amount) public override returns (bool) {
        require(!frozenAccounts[from], 'Sender account is frozen');
        require(!frozenAccounts[to], 'Recipient account is frozen');
        return super.transferFrom(from, to, amount);
    }

    function approve(address spender, uint256 amount) public override returns (bool) {
        require(!frozenAccounts[msg.sender], 'Account is frozen');
        require(!frozenAccounts[spender], 'Spender account is frozen');
        return super.approve(spender, amount);
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

    function _stringsEqual(string memory a, string memory b) internal pure returns (bool) {
        return bytes(a).length == bytes(b).length && keccak256(bytes(a)) == keccak256(bytes(b));
    }

    function _getChainID() internal view returns (uint256) {
        uint256 id;
        assembly {
            id := chainid()
        }
        return id;
    }
}
