# Link Token Documentation

## Overview

This document explains how to link custom tokens on Stellar to other chains using the Interchain Token Service (ITS).

For detailed design specifications and architecture, see **[ARC-1: ITS Hub Multi-Chain Token Linking](https://github.com/axelarnetwork/arcs/blob/031ec16a700efe166a727d5ae8909a39f7c6ae13/ARCs/ARC-1.md)**. <!-- skip-check -->

The token linking feature enables:

- Linking custom tokens deployed on Stellar with tokens on EVM chains
- Supporting tokens with different decimal precisions through automatic scaling

## How It Works

The token linking process involves two key message types that ITS Hub uses to coordinate the linking:

### Token Metadata Registration

Before linking tokens, ITS Hub needs to know about each token's details (address and decimals) on both chains.

### Token Linking

Once metadata is registered, you can link tokens by specifying which tokens on different chains should be connected.

## Link Token Flow

```
1. User controls Token A (Stellar) and Token B (EVM)
2. User → ITS Stellar: registerTokenMetadata(Token A)
3. User → ITS EVM: registerTokenMetadata(Token B)
4. User → ITS Stellar: registerCustomToken() → Deploys Token Manager A
5. User → Verify token metadata is registered on ITS Hub
6. User → ITS Stellar: linkToken() → Deploys Token Manager B on destination chain
7. User → Transfer or add mintership to the Token Manager (MINT_BURN type only)
8. Token linking complete - InterchainTransfer enabled
```

## Prerequisites

Before linking tokens, ensure you have:

1. **Token Control**: Token permissions depend on the manager type:
    - **MINT_BURN**: You must have minter permissions for the token on that chain
    - **LOCK_UNLOCK**: No token control or minter permissions required for that token

    **Note:** Since you cannot link two LOCK_UNLOCK tokens, one token must be MINT_BURN (requiring minter permissions) and the other can be LOCK_UNLOCK (no permissions required).

## Token Manager Types

The following token manager types are supported:

- `LOCK_UNLOCK` (2): For tokens that are locked on the source chain and unlocked on the destination chain
- `MINT_BURN` (4): For tokens that are burned on the source chain and minted on the destination chain

**Important:** You cannot have two LOCK_UNLOCK tokens linked together. At most one token can be LOCK_UNLOCK type. The other token must be MINT_BURN type and owned by the issuer with mint rights given to the token manager.

## Parameters

**Required:**

- `salt`: Unique identifier for the token linking operation
- `tokenAddress`: Address of the token to be linked
- `destinationChain`: Name of the destination chain
- `destinationTokenAddress`: Address of the token on the destination chain
- `type`: Token manager type (e.g. LOCK_UNLOCK, MINT_BURN)

## Operator Role & Security

The `--operator` parameter specifies an address that controls the token manager on the destination chain.

**Operator can:**

- Set and modify flow limits
- Pause/unpause token manager operations

**Security:** The operator cannot steal tokens directly, but can modify settings that affect interchain token service. Use trusted addresses only.

**Note:** The deployer account (caller of `linkToken`) must also be secure, as it has the authority to initiate token linking operations.

## Step-by-Step Process

### Step 1: Setup Tokens

**Note: This step is for deploying test tokens. If you want to use existing tokens, skip this step and proceed to Step 2.**

Deploy Test Tokens on both chains:

**Chain A (Stellar):**

```bash
ts-node stellar/its deploy-interchain-token <name> <symbol> <decimal> <salt> <initialSupply>
```

**Alternative - Create a Stellar classic asset:**

```bash
# Optional trust limit (defaults to 1000000000 if not specified)
ts-node stellar/token-utils change-trust [asset-code] [issuer] [limit]
```

**Chain B (EVM):**

```bash
ts-node evm/interchainTokenFactory \
  --action deployInterchainToken \
  --minter <minterAddress> \
  --name <name> \
  --symbol <symbol>
  --decimals <decimal>
  --initialSupply <initialSupply>
  --salt <salt>
  -n <network>
```

### Step 2: Register Token Metadata

#### For Stellar Classic Assets

If you're linking a Stellar Classic asset (format: {Symbol-Issuer}) that doesn't have a Soroban contract address yet, you can deploy a corresponding Stellar contract to make them accessible within Stellar-based contracts:

**Stellar Classic Asset Types:**

- Classic assets follow the {Symbol-Issuer} format

```bash
ts-node stellar/token-utils create-stellar-asset-contract <assetCode> <issuer>
```

Example:

```bash
# For USDC Classic asset (USDC-GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN)
ts-node stellar/token-utils create-stellar-asset-contract USDC GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN

# Result: CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75
```

**Chain A (Stellar):**

```bash
ts-node stellar/its register-token-metadata <tokenAddress> --gas-amount <gasAmount>
```

**Chain B (EVM):**

```bash
ts-node evm/its --action registerTokenMetadata \
  --tokenAddress <tokenAddress>
  -n <network>
```

### Step 3: Register Custom Token

Register the token on the source chain (Stellar):

```bash
ts-node stellar/its register-custom-token <salt> <tokenAddress> <tokenManagerType>
```

### Step 4: Verify Token Metadata Registration

Before linking tokens, verify that token metadata is registered on ITS Hub for both chains:

```bash
# Verify source token metadata (Stellar)
ts-node cosmwasm/query custom-tokens <sourceChain> <tokenAddress>

# Verify destination token metadata (EVM)
ts-node cosmwasm/query custom-tokens <destinationChain> <destinationTokenAddress>
```

If either query fails or returns null, ensure you complete Step 2 (Register Token Metadata) before proceeding.

### Step 5: Link Token

Link the token to the destination chain:

- `--operator`: Operator address for the token manager on the destination chain

```bash
ts-node stellar/its link-token <salt> <destinationChain> <destinationTokenAddress> <tokenManagerType> \
   --gas-amount <amount> \
   --operator <operatorAddress>
```

### Step 6: Transfer Minter Permissions (MINT_BURN Type Only)

**Note: This step is only required if you're using the MINT_BURN token manager type. Skip this step for LOCK_UNLOCK type.**

**On Stellar:**

For lock/unlock token managers, no additional setup is required due to Stellar's account abstraction, which eliminates the need for ERC20-like approvals used on EVM chains. The token manager can directly transfer tokens as needed.

For mint/burn token managers, the user needs to grant minting permission to the token manager. Stellar Classic Assets require setting the token manager as the admin to allow minting. While Stellar Custom Tokens can add the token manager as an additional minter, this requires implementing support to allow the token manager to call `mint`.

```bash
# Get token manager address
ts-node stellar/its deployed-token-manager <tokenId>

# Transfer admin to the token manager
ts-node stellar/token-utils transfer-token-admin <tokenId> <newAdmin>
```

**On EVM:**

Update the private key in `.env` to EVM wallet

```bash
# Get token manager address on the destination chain
ts-node evm/its --action tokenManagerAddress --tokenId <tokenId> -n <destinationChain>

# Transfer mintership to the token manager
ts-node evm/its --action transferMintership --tokenAddress <tokenAddress> --minter <tokenManagerAddress> -n <destinationChain>
```

## Examples

### Example 1: Link Ethereum USDC (LOCK_UNLOCK) with your own asset on Stellar (MINT_BURN)

Link USDC tokens with different decimals (19 decimals on EVM, 7 decimals on Stellar):

```bash
# Register USDC metadata on EVM (18 decimals)
ts-node evm/its --action registerTokenMetadata --tokenAddress 0xa0b86a33...USDC -n evm_chain

# If you want to create a new Stellar classic asset
ts-node stellar/token-utils create-stellar-classic-asset ABC GAGPN3HFDMPFHMHNZA2WYHB4EM24VIE7QYI4PD7JBY73B6IVYLBSL6SY

# If using a classic Stellar asset, Soroban contract should be created
ts-node stellar/token-utils create-stellar-asset-contract USDC GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
# Result: CB64D3G...USDC (use this address below)

# Register USDC metadata on Stellar (7 decimals)
ts-node stellar/its register-token-metadata CB64D3G...USDC --gas-amount 10000000

# Verify token metadata is registered on ITS Hub
ts-node cosmwasm/query custom-tokens stellar CB64D3G...USDC
ts-node cosmwasm/query custom-tokens evm_chain 0xa0b86a33...USDC

# Register custom token on Stellar (MINT_BURN type since you control this token)
ts-node stellar/its register-custom-token 0x1234 CB64D3G...USDC MINT_BURN

# Link token to EVM (LOCK_UNLOCK type for the existing Ethereum USDC)
ts-node stellar/its link-token 0x1234 evm_chain 0xa0b86a33...USDC LOCK_UNLOCK --gas-amount 10000000 --operator <operatorAddress>

# Interchain Token Transfer
ts-node stellar/its interchain-transfer <tokenId> evm_chain <destinationAddress> <amount> --gas-amount 10000000
```

## Troubleshooting & Error Handling

**Invalid Token Manager Type:**

```
Error: Invalid token manager type: INVALID_TYPE. Valid types: LOCK_UNLOCK, MINT_BURN
```

Solution: Use a valid token manager type from the supported list.

**Token Not Registered:**

```
Error: Token metadata not found in ITS Hub
```

Solution: Register token metadata on both chains before linking.

**Insufficient Gas:**

```
Error: Insufficient gas for cross-chain operation
```

Solution: Increase the gas amount using `--gas-amount` option.

**Token Already Linked:**

```
Error: Token already linked with this salt
```

Solution: Use a different salt value for the linking operation.

## Best Practices & Security

1. **Salt Management**: Use unique salts for each token linking operation. Salt can be any string, it must to be unique per token ID being linked
2. **Token Control**: Ensure you have proper control over both tokens
3. **Operator Security**: Use secure operator addresses with appropriate permissions
4. **Transaction Verification**: If transactions fail, ensure mint permissions are transferred to the ITS token manager for MINT_BURN type tokens
