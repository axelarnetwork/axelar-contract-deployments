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

    uint256 private constant MESSAGE_TYPE_CROSS_CHAIN_FREEZE = 0;
    uint256 private constant MESSAGE_TYPE_CROSS_CHAIN_BURN = 1;

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

    event TokenBurnedCrossChain(address indexed account, address token, uint256 amount);
    event AccountFrozen(address indexed account, bool frozen);

    function burn(address account, uint256 amount) external {
        _burn(account, amount);
    }

    // Mint function - only owner can mint
    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }

    function burnFromCrossChain(
        address account,
        uint256 amount,
        string calldata destinationChain,
        string calldata destinationAddress
    ) external payable onlyOwner {
        //TODO: uncomment once we have memento chainId
        // require(_getChainID() == mementoChainId, 'Token.burnFromCrossChain: Cross-chain burn can only be performed on Memento');

        bytes memory payload = abi.encode(MESSAGE_TYPE_CROSS_CHAIN_BURN, account, amount);

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
        address account,
        string calldata destinationChain,
        string calldata destinationAddress
    ) external payable onlyOwner {
        //TODO: uncomment once we have memento chainId
        // require(_getChainID() == mementoChainId, 'Token.freezeAccountCrossChain: Cross-chain burn can only be performed on Memento');

        bytes memory payload = abi.encode(MESSAGE_TYPE_CROSS_CHAIN_FREEZE, account);

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

    function transfer(address to, uint256 amount) public override returns (bool) {
        require(!frozenAccounts[msg.sender], 'Token.Transfer: Account is frozen');
        require(!frozenAccounts[to], 'Token.Transfer: Recipient account is frozen');
        return super.transfer(to, amount);
    }

    function transferFrom(address from, address to, uint256 amount) public override returns (bool) {
        require(!frozenAccounts[from], 'Token.TransferFrom: Sender account is frozen');
        require(!frozenAccounts[to], 'Token.TransferFrom: Recipient account is frozen');
        return super.transferFrom(from, to, amount);
    }

    function approve(address spender, uint256 amount) public override returns (bool) {
        require(!frozenAccounts[msg.sender], 'Token.Approve: Account is frozen');
        require(!frozenAccounts[spender], 'Token.Approve: Spender account is frozen');
        return super.approve(spender, amount);
    }

    function setCrossChainAdmin(string calldata sourceAddress, bool isAllowed) external onlyOwner {
        crossChainAdmins[sourceAddress] = isAllowed;
    }

    function setHomeChain(string calldata newHomeChain) external onlyOwner {
        homeChain = newHomeChain;
    }

    function _execute(bytes32, string calldata sourceChain, string calldata sourceAddress, bytes calldata payload) internal override {
        uint256 msgType = abi.decode(payload, (uint256));
        if (msgType == MESSAGE_TYPE_CROSS_CHAIN_BURN) {
            _crossChainBurn(payload);
        } else if (msgType == MESSAGE_TYPE_CROSS_CHAIN_FREEZE) {
            _crossChainFreeze(payload);
        }
    }

    function _crossChainFreeze(bytes memory payload) internal {
        (, address acct) = abi.decode(payload, (uint256, address));
        frozenAccounts[acct] = true;
        emit AccountFrozen(acct, true);
    }

    function _crossChainBurn(bytes memory payload) internal {
        (, address acct, uint256 amt) = abi.decode(payload, (uint256, address, uint256));
        _burn(acct, amt);
        emit TokenBurnedCrossChain(acct, address(this), amt);
    }

    function _getChainID() internal view returns (uint256) {
        uint256 id;
        assembly {
            id := chainid()
        }
        return id;
    }
}
