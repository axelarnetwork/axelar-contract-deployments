// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

// Importing just so the `forge build` picks it up and produces the artifact for us.
import {GatewayCaller} from "interchain-token-service/contracts/utils/GatewayCaller.sol";
import {InterchainTokenDeployer} from "interchain-token-service/contracts/utils/InterchainTokenDeployer.sol";
import {InterchainTokenFactory} from "interchain-token-service/contracts/InterchainTokenFactory.sol";
import {InterchainTokenService} from "interchain-token-service/contracts/InterchainTokenService.sol";
import {InterchainToken} from "interchain-token-service/contracts/interchain-token/InterchainToken.sol";
import {TokenHandler} from "interchain-token-service/contracts/TokenHandler.sol";
import {TokenManagerDeployer} from "interchain-token-service/contracts/utils/TokenManagerDeployer.sol";
import {InterchainProxy} from "interchain-token-service/contracts/proxies/InterchainProxy.sol";
import {TokenManager} from "interchain-token-service/contracts/token-manager/TokenManager.sol";
import {Create3Deployer} from "@axelar-network/axelar-gmp-sdk-solidity/contracts/deploy/Create3Deployer.sol";
import {Create3Deployer} from "@axelar-network/axelar-gmp-sdk-solidity/contracts/deploy/Create3Deployer.sol";

import {ERC20} from "openzeppelin-contracts/contracts/token/ERC20/ERC20.sol";
import {Minter} from "interchain-token-service/contracts/utils/Minter.sol";
import {IERC20MintableBurnable} from "interchain-token-service/contracts/interfaces/IERC20MintableBurnable.sol";

/**
 * /**
 * @title InterchainToken
 * @notice This contract implements an interchain token which extends InterchainToken functionality.
 * @dev This contract also inherits Minter and Implementation logic.
 */
contract TestCanonicalToken is ERC20, Minter, IERC20MintableBurnable {
    uint8 internal immutable _decimals;

    uint256 internal constant UINT256_MAX = 2 ** 256 - 1;

    /**
     * @notice Constructs the InterchainToken contract.
     * @dev Makes the implementation act as if it has been setup already to disallow calls to init() (even though that would not achieve anything really).
     */
    constructor(
        string memory _name,
        string memory _symbol,
        uint8 _decimalsValue
    ) ERC20(_name, _symbol) {
        _decimals = _decimalsValue;
        _addMinter(msg.sender);
    }

    function decimals() public view override returns (uint8) {
        return _decimals;
    }

    /**
     * @notice Function to mint new tokens.
     * @dev Can only be called by the minter address.
     * @param account The address that will receive the minted tokens.
     * @param amount The amount of tokens to mint.
     */
    function mint(
        address account,
        uint256 amount
    ) external onlyRole(uint8(Roles.MINTER)) {
        _mint(account, amount);
    }

    /**
     * @notice Function to burn tokens.
     * @dev Can only be called by the minter address.
     * @param account The address that will have its tokens burnt.
     * @param amount The amount of tokens to burn.
     */
    function burn(
        address account,
        uint256 amount
    ) external onlyRole(uint8(Roles.MINTER)) {
        _burn(account, amount);
    }

    function addMinter(address account) external onlyRole(uint8(Roles.MINTER)) {
        _addMinter(account);
    }
}
