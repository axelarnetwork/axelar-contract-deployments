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
1. User has access to Token A (Source Chain) and Token B (Destination Chain)
2. User → ITS Source Chain: registerTokenMetadata(Token A)
3. User → ITS Destination Chain: registerTokenMetadata(Token B)
4. User → ITS Source Chain: registerCustomToken() → Deploys Token Manager A on Source Chain
5. User → ITS Source Chain: linkToken() → Deploys Token Manager B on Destination Chain
6. User → Transfer or add mintership to the Token Managers (MINT_BURN and MINT_BURN_FROM type only)
7. Token linking complete - InterchainTransfer enabled
```

## Prerequisites

Before linking tokens, ensure you have:

- **Token Control**: Token permissions depend on the manager type:
    - **LOCK_UNLOCK**: No token control or minter permissions required for that token
    - **MINT_BURN_FROM / MINT_BURN**: You must have permission to transfer or add mintership for the token on that chain

## Token Manager Types

The following token manager types are supported:

- `MINT_BURN_FROM` (1): For tokens that are burned/minted using burnFrom/mint functions on the chain
- `LOCK_UNLOCK` (2): For tokens that are locked/unlocked on the chain
- `MINT_BURN` (4): For tokens that are burned/minted on the chain

**Important:** Linking two LOCK_UNLOCK tokens is not recommended. One token should be MINT_BURN or MINT_BURN_FROM (requiring minter permissions) and the other can be LOCK_UNLOCK (no permissions required). Using MINT_BURN or MINT_BURN_FROM on both sides is supported.

## Parameters

**Required:**

- `salt`: Unique identifier for the token linking operation. Used to generate a unique `tokenId`
- `tokenAddress`: Address of the token to be linked
- `destinationChain`: Name of the destination chain
- `destinationTokenAddress`: Address of the token on the destination chain
- `type`: Token manager type (e.g. LOCK_UNLOCK, MINT_BURN, MINT_BURN_FROM)

## Operator Role & Security

The `--operator` parameter specifies an address that controls the token manager on the destination chain.

**Operator can:**

- Set and modify flow limits
- Pause/unpause token manager operations

**Security:** The operator cannot steal tokens directly, but can modify settings that affect interchain token service. Use trusted addresses only.

**Note:** The deployer account (caller of `linkToken`) must also be secure, as it has the authority to initiate token linking operations.

## Step-by-Step Process

**Example Configuration:**

- Chain A (EVM): Source chain using LOCK_UNLOCK token manager type
- Chain B (Stellar): Destination chain using MINT_BURN or MINT_BURN_FROM token manager type

### Step 1: Setup Tokens

**Note: This step is for deploying test tokens. If you want to use existing tokens, skip this step and proceed to Step 2.**

Deploy Test Tokens on both chains:

**Chain A (EVM):**

**Note:** On EVM, we're deploying a non-custom Interchain Token from ITS, which creates a new token contract managed by the Interchain Token Service.

Update the private key in `.env` to EVM wallet

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

**Chain B (Stellar):**

**Note:** On Stellar, we're deploying a customized pre-deployed token contract that you control directly, in contrast to the ITS-managed token on EVM.

```bash
# Create a custom token
# Note: Custom token implementation can vary based on your requirements.
ts-node stellar/its create-custom-token TEST TEST 7
```

**Alternative - Create a Stellar classic asset by setting trustline:**

> Stellar Classic Asset Trustline: https://developers.stellar.org/docs/tokens/stellar-asset-contract#interacting-with-classic-stellar-assets

```bash
# Optional trust limit (defaults to 1000000000 if not specified)
ts-node stellar/token-utils change-trust [asset-code] [issuer] [limit]
```

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

### Step 2: Register Token Metadata

**Chain A (EVM):**

```bash
ts-node evm/its register-token-metadata <tokenAddress> -n <network> --gasValue <gasAmount> -y
```

**Chain B (Stellar):**

```bash
ts-node stellar/its register-token-metadata <tokenAddress> --gas-amount <gasAmount>
```

### Step 3: Register Custom Token

Register the token on the source chain (EVM):

```bash
ts-node evm/interchainTokenFactory.js \
  --action registerCustomToken \
  --tokenAddress <tokenAddress> \
  --tokenManagerType LOCK_UNLOCK \
  --operator <operator> \
  --salt <salt> \
  -n <network>
```

### Step 4: Link Token

Link the token to the destination chain:

- `--operator`: Operator address for the token manager on the destination chain

```bash
ts-node evm/interchainTokenFactory.js \
  --action linkToken \
  --destinationChain <destinationChain> \
  --destinationTokenAddress <destinationTokenAddress> \
  --tokenManagerType MINT_BURN (# or MINT_BURN_FROM) \
  --linkParams "0x" \
  --salt <salt>> \
  -n <network> \
  --gasValue <gasAmount>
```

### Step 5: Transfer or Add Minter Permissions (MINT_BURN and MINT_BURN_FROM Types Only)

**Note: This step is only required if you're using the MINT_BURN or MINT_BURN_FROM token manager types. Skip this step for LOCK_UNLOCK type.**

**On EVM:**

```bash
# Get token manager address on the destination chain
ts-node evm/its token-manager-address <tokenId> -n <network>

# Transfer mintership to the token manager
ts-node evm/its transfer-mintership <tokenAddress> -n <network>
```

**On Stellar:**

For MINT_BURN_FROM token managers, you need to add the token manager as a minter. For MINT_BURN token managers, you need to set the token manager as admin. Stellar Classic Assets require setting the token manager as the admin to allow minting.

For LOCK_UNLOCK token managers, no additional setup is required due to Stellar's account abstraction, which eliminates the need for ERC20-like approvals used on EVM chains. The token manager can directly transfer tokens as needed.

**Important:** Based on your custom token implementation, choose the right token manager type. If your token supports only `mint()` and `burn()` functions that require admin privileges, use MINT_BURN. If your token supports `mint_from()` and `burn()` functions that work with minter permissions, use MINT_BURN_FROM.

**Technical Details:**

- **MINT_BURN** requires admin role because the token manager must call the `mint(env: Env, to: Address, amount: i128)` function.
- **MINT_BURN_FROM** requires minter role because it uses the `mint_from(env: Env, minter: Address, to: Address, amount: i128)` function. This function can be called by any authorized minter without requiring full admin privileges.

```bash
# Get the token manager address
ts-node stellar/its deployed-token-manager <tokenId>

# Set admin to the token manager if you are using the MINT_BURN type
# Note: MINT_BURN requires admin role because the token manager must call the mint() function
ts-node stellar/token-utils set-admin <tokenAddress> <tokenManagerAddress>

# Alternatively, add minter if you are using the MINT_BURN_FROM type
# Note: MINT_BURN_FROM requires minter role because it uses the mint_from() function
# Adding minter permissions can vary based on your custom token implementation
ts-node stellar/token-utils add-minter <tokenAddress> <tokenManagerAddress>
```

### Optional: Transfer Token Admin (MINT_BURN Type Only)

**Note: This step is optional and only applicable when using the MINT_BURN token manager type. For MINT_BURN_FROM and LOCK_UNLOCK types, skip this step.**

For `MINT_BURN` token managers, once you transfer admin to the token manager, you need to request ITS owner to transfer the admin back to you.

**On Stellar:**

```bash
# Transfer token admin to the new admin address (only ITS Owner is allowed to call transfer-token-admin)
ts-node stellar/token-utils transfer-token-admin <tokenId> <adminAddress>
```

## Examples

### Example 1: Link USDC on EVM (Source - LOCK_UNLOCK) with Stellar Classic Asset (Destination - MINT_BURN)

Link USDC tokens with different decimals (18 decimals on EVM, 7 decimals on Stellar):

```bash
# If you want to change trust limit for a Stellar classic asset
SYMBOL=USDC
ISSUER=GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN
ts-node stellar/token-utils change-trust $SYMBOL $ISSUER 10000000

# If using a classic Stellar asset, Soroban contract should be created
ts-node stellar/token-utils create-stellar-asset-contract $SYMBOL $ISSUER
# Result: CB64D3G...USDC (STELLAR_TOKEN_ADDRESS)

# Register USDC metadata on Stellar (7 decimals)
STELLAR_TOKEN_ADDRESS="STELLAR_TOKEN_ADDRESS"
ts-node stellar/its register-token-metadata $STELLAR_TOKEN_ADDRESS --gas-amount 10000000

# Register USDC metadata on EVM (18 decimals)
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"
EVM_CHAIN_NAME="EVM_CHAIN_NAME"
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN_NAME --gasValue 1000000000000000000

# Register custom token - LOCK_UNLOCK type (2) on EVM
LOCK_UNLOCK_TYPE=2
SALT=0x1234
ts-node evm/interchainTokenFactory.js \
  --action registerCustomToken \
  --tokenAddress $EVM_TOKEN_ADDRESS \
  --tokenManagerType $LOCK_UNLOCK_TYPE \
  --operator 0x1234... \
  --salt $SALT \
  -n $EVM_CHAIN_NAME

# Link token - MINT_BURN type (4) on Stellar
MINT_BURN_TYPE=4
ts-node evm/interchainTokenFactory.js \
  --action linkToken \
  --destinationChain stellar \
  --destinationTokenAddress $STELLAR_TOKEN_ADDRESS \
  --tokenManagerType $MINT_BURN_TYPE \
  --linkParams "0x" \
  --salt $SALT \
  -n $EVM_CHAIN_NAME \
  --gasValue 10000000000000000000
# Result: 0x89a0...abcd (TOKEN_ID)

# Get token manager address on Stellar
TOKEN_ID="TOKEN_ID"
ts-node stellar/its deployed-token-manager $TOKEN_ID
# Result: CATE...ABCD (STELLAR_TOKEN_MANAGER)

# Transfer admin to the token manager on Stellar
STELLAR_TOKEN_MANAGER="STELLAR_TOKEN_MANAGER"
ts-node stellar/token-utils set-admin $STELLAR_TOKEN_ADDRESS $STELLAR_TOKEN_MANAGER

# Interchain Token Transfer from EVM to Stellar
STELLAR_DESTINATION_ADDRESS="STELLAR_DESTINATION_ADDRESS"
TRANSFER_AMOUNT=1
ts-node evm/its interchain-transfer stellar $TOKEN_ID $STELLAR_DESTINATION_ADDRESS $TRANSFER_AMOUNT -n $EVM_CHAIN_NAME --gasValue 10000000000000000000

# Interchain Token Transfer from Stellar to EVM
EVM_DESTINATION_ADDRESS="EVM_DESTINATION_ADDRESS"
ts-node stellar/its interchain-transfer $TOKEN_ID $EVM_CHAIN_NAME $EVM_DESTINATION_ADDRESS $TRANSFER_AMOUNT --gas-amount 10000000
```

### Example 2: Link Custom Token on Stellar (Source - MINT_BURN_FROM) with USDC on EVM (Destination - LOCK_UNLOCK)

Link tokens with different decimals (7 decimals on Stellar, 18 decimals on EVM):

```bash
# Register USDC metadata on EVM (18 decimals)
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"
EVM_CHAIN_NAME="EVM_CHAIN_NAME"
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN_NAME --gasValue 1000000000000000000

# Register Custom Token metadata on Stellar (7 decimals)
STELLAR_TOKEN_ADDRESS="STELLAR_TOKEN_ADDRESS"
ts-node stellar/its register-token-metadata $STELLAR_TOKEN_ADDRESS --gas-amount 10000000

# Register custom token - MINT_BURN_FROM type on Stellar
SALT=0x1234
ts-node stellar/its register-custom-token $SALT $STELLAR_TOKEN_ADDRESS MINT_BURN_FROM

# Link token - LOCK_UNLOCK type (2) on EVM
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"
ts-node stellar/its link-token $SALT $EVM_CHAIN_NAME $EVM_TOKEN_ADDRESS LOCK_UNLOCK --gas-amount 10000000
# Result: 0x89a0...abcd (TOKEN_ID)

# Get token manager address on Stellar
TOKEN_ID="TOKEN_ID"
ts-node stellar/its deployed-token-manager $TOKEN_ID
# Result: CATE...ABCD (STELLAR_TOKEN_MANAGER)

# Add the token manager as a minter on Stellar
STELLAR_TOKEN_MANAGER="STELLAR_TOKEN_MANAGER"
ts-node stellar/token-utils add-minter $STELLAR_TOKEN_ADDRESS $STELLAR_TOKEN_MANAGER

# Interchain Token Transfer EVM to Stellar
STELLAR_DESTINATION_ADDRESS="STELLAR_DESTINATION_ADDRESS"
TRANSFER_AMOUNT=1
ts-node evm/its interchain-transfer stellar $TOKEN_ID $STELLAR_DESTINATION_ADDRESS $TRANSFER_AMOUNT -n $EVM_CHAIN_NAME --gasValue 10000000000000000000

# Interchain Token Transfer from Stellar to EVM
EVM_DESTINATION_ADDRESS="EVM_DESTINATION_ADDRESS"
ts-node stellar/its interchain-transfer $TOKEN_ID $EVM_CHAIN_NAME $EVM_DESTINATION_ADDRESS $TRANSFER_AMOUNT --gas-amount 10000000
```

## Troubleshooting & Error Handling

**Invalid Token Manager Type:**

```
Error: Invalid token manager type: INVALID_TYPE. Valid types: LOCK_UNLOCK, MINT_BURN, MINT_BURN_FROM
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
4. **Mint Permission**: If interchain transfer fails, ensure mint permissions are transferred to the ITS token manager for MINT_BURN and MINT_BURN_FROM type tokens
