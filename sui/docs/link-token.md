# Link Token Documentation

## Overview

This document explains how to link custom tokens on Sui to other chains using the Interchain Token Service (ITS).

For detailed design specifications and architecture, see **[ARC-1: ITS Hub Multi-Chain Token Linking](https://github.com/axelarnetwork/arcs/blob/main/ARCs/ARC-1.md)**. <!-- skip-check -->

The token linking feature enables:

- Linking custom tokens deployed on Sui with tokens on trusted chains
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
2. User → ITS Source Chain: registerCoinMetadata(Token A)
3. User → ITS Destination Chain: registerTokenMetadata(Token B)
4. User → ITS Source Chain: registerCustomCoin() → Deploys Token Manager A on Source Chain
5. User → ITS Source Chain: linkCoin() → Deploys Token Manager B on Destination Chain
6. User → Transfer TreasuryCap to the Token Manager (MINT_BURN type only)
7. Token linking complete - InterchainTransfer enabled
```

## Prerequisites

Before linking tokens, ensure you have:

- **Token Control**: Token permissions depend on the manager type:
    - **LOCK_UNLOCK**: No token control or minter permissions required for that token
    - **MINT_BURN**: You must have the TreasuryCap for the token on Sui to transfer it to the token manager

## Token Manager Types

For Sui, only the following token manager types are supported:

- `LOCK_UNLOCK` (2): For tokens that are locked/unlocked on the chain
- `MINT_BURN` (4): For tokens that are burned/minted on the chain

**Important:** Linking two LOCK_UNLOCK tokens is not recommended. One token should be MINT_BURN (requiring TreasuryCap transfer if token deployed on Sui) and the other can be LOCK_UNLOCK (no permissions required). Using MINT_BURN on both sides is supported.

## Parameters

```move
public fun link_coin(
        self: &InterchainTokenService,
        deployer: &Channel,
        salt: Bytes32,
        destination_chain: String,
        destination_token_address: vector<u8>,
        token_manager_type: TokenManagerType,
        link_params: vector<u8>,
    ): MessageTicket { ... }
```

**Required:**

- `deployer`: An ITS `Channel` to represent the deployer of the coin. Transaction sender's Sui address will not be tracked, only their `Channel`
- `salt`: Unique identifier for the token linking operation. Used to generate a unique `tokenId`. On Sui, the salt must be 64 characters (e.g.32 bytes) matching the Sui address format
- `destination_chain`: Name of the destination chain (e.g., `avalanche`, `ethereum`)
- `destination_token_address`: Address of the token on the destination chain
- `token_manager_type`: Token manager type on Sui (e.g., `2_u256` for LOCK_UNLOCK or, `4_u256` for MINT_BURN)
- `link_params`: Bytes representation of an address on the destination chain that is an operator of the destination token

## Step-by-Step Process

**Example Configuration:**

- Chain A (Sui): Source chain using LOCK_UNLOCK token manager type
- Chain B (EVM): Destination chain using MINT_BURN token manager type

### Step 1: Setup Tokens

Use your existing token contract addresses. If you need to create test tokens for demonstration purposes, expand the section below.

<details>
    <summary><strong>Deploy Test Tokens (for testing/demo only)</strong></summary>
</details>

Deploy test tokens on both chains:

**Chain A (Sui):**

```bash
# Deploy token on Sui
ts-node sui/tokens publish-coin <SYMBOL> <NAME> <DECIMALS>
```

**Chain B (EVM):**

**Note:** On EVM, we're deploying a non-custom Interchain Token from ITS, which creates a new token contract managed by the Interchain Token Service.

Update the private key in `.env` to EVM wallet

```bash
ts-node evm/interchainTokenFactory \
  --action deployInterchainToken \
  --minter <minterAddress> \
  --name <name> \
  --symbol <symbol> \
  --decimals <decimals> \
  --initialSupply <initialSupply> \
  --salt <salt> \
  -n <network>
```

### Step 2: Register Token Metadata

**Chain A (Sui):**

```bash
ts-node sui/its register-coin-metadata <SYMBOL>
```

**Chain B (EVM):**

```bash
ts-node evm/its register-token-metadata <tokenAddress> -n <network>
```

### Step 3: Register Custom Token

Register the token on the source chain (Sui):

```bash
# For LOCK_UNLOCK token manager
ts-node sui/its register-custom-coin <SYMBOL> <NAME> <DECIMALS> --salt <SALT>

# For MINT_BURN token manager (requires --treasuryCap flag)
ts-node sui/its register-custom-coin <SYMBOL> <NAME> <DECIMALS> --salt <SALT> --treasuryCap
```

Record the channel ID from the result for use in the next step.

### Step 4: Link Token

Link the token to the destination chain:

```bash
ts-node sui/its link-coin <SYMBOL> <DESTINATION_CHAIN> <DESTINATION_TOKEN_ADDRESS> \
    --tokenManagerMode <lock_unlock|mint_burn> \
    --destinationTokenManagerMode <lock_unlock|mint_burn> \
    --channel <CHANNEL> \
    --registered
```

Record the Token ID from the result.

### Step 5: Transfer TreasuryCap or Minter Permissions

**Note: This step is only required if you're using the MINT_BURN token manager type on the destination chain (EVM). Skip this step for LOCK_UNLOCK type.**

**On EVM (if destination uses MINT_BURN):**

```bash
# Get token manager address on the destination chain
ts-node evm/its token-manager-address <tokenId> -n <network>

# Transfer mintership to the token manager
ts-node evm/its transfer-mintership <tokenAddress> <tokenManagerAddress> -n <network>
```

**On Sui (if source uses MINT_BURN):**

For MINT_BURN token managers on Sui, you need to transfer the TreasuryCap to the token manager during the registration process (using the `--treasuryCap` flag in Step 3). No additional setup is required after linking.

For LOCK_UNLOCK token managers on Sui, no additional setup is required due to Sui's account abstraction and object model, which eliminates the need for ERC20-like approvals used on EVM chains. The token manager can directly transfer tokens as needed.

## Examples

### Example 1: Link Token on Sui (Source - LOCK_UNLOCK) with EVM Token (Destination - MINT_BURN)

Link tokens with different decimals (6 decimals on Sui, 9 decimals on EVM):

```bash
# Common variables
NAME="TEST1234"
SYMBOL="TEST1234"
EVM_DECIMALS=9
SUI_DECIMALS=6
EVM_CHAIN=avalanche
SALT=0x0000000000000000000000000000000000000000000000000000000000000001
EVM_TEMP_SALT="TEST1234"
EVM_WALLET_ADDRESS="0x13f8C723AeB8CA762c652c553a11a11483846d8B"
SUI_WALLET_ADDRESS="0x76f89a9b56dc580aed9f97e2b3bd03d8d24464e38522da9464c15103761c6707"

# Deploy token on EVM
ts-node evm/interchainTokenFactory --action deployInterchainToken \
    --minter $EVM_WALLET_ADDRESS \
    --name $NAME \
    --symbol $SYMBOL \
    --decimals $EVM_DECIMALS \
    --initialSupply 100000000000 \
    --salt $EVM_TEMP_SALT \
    -n $EVM_CHAIN
# Record EVM_TOKEN_ADDRESS from result

# Register EVM token metadata
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN

# Deploy and register token on Sui with LOCK_UNLOCK mode
ts-node sui/its register-custom-coin $SYMBOL $NAME $SUI_DECIMALS --salt $SALT
# Record CHANNEL from result

# Register Sui token metadata
ts-node sui/its register-coin-metadata $SYMBOL

# Link Sui token to EVM token
ts-node sui/its link-coin $SYMBOL $EVM_CHAIN $EVM_TOKEN_ADDRESS \
    --tokenManagerMode lock_unlock \
    --destinationTokenManagerMode mint_burn \
    --channel $CHANNEL \
    --registered
# Record SUI_TOKEN_ID from result

# Get token manager address on EVM
ts-node evm/its token-manager-address $SUI_TOKEN_ID -n $EVM_CHAIN
# Record TOKEN_MANAGER_ADDRESS from result

# Transfer mintership to token manager
ts-node evm/its transfer-mintership $EVM_TOKEN_ADDRESS $TOKEN_MANAGER_ADDRESS -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $SUI_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS [amount]

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $SUI_TOKEN_ID $SUI_WALLET_ADDRESS 125 -n $EVM_CHAIN
```

### Example 2: Link Token on Sui (Source - MINT_BURN) with EVM Token (Destination - LOCK_UNLOCK)

Link tokens with different decimals (6 decimals on Sui, 9 decimals on EVM):

```bash
# Common variables
NAME="TEST1234"
SYMBOL="TEST1234"
EVM_DECIMALS=9
SUI_DECIMALS=6
EVM_CHAIN=avalanche
SALT=0x0000000000000000000000000000000000000000000000000000000000000001
EVM_TEMP_SALT="TEST1234"
EVM_WALLET_ADDRESS="0x13f8C723AeB8CA762c652c553a11a11483846d8B"
SUI_WALLET_ADDRESS="0x76f89a9b56dc580aed9f97e2b3bd03d8d24464e38522da9464c15103761c6707"

# Deploy token on EVM
ts-node evm/interchainTokenFactory --action deployInterchainToken \
    --minter $EVM_WALLET_ADDRESS \
    --name $NAME \
    --symbol $SYMBOL \
    --decimals $EVM_DECIMALS \
    --initialSupply 100000000000 \
    --salt $EVM_TEMP_SALT \
    -n $EVM_CHAIN
# Record EVM_TOKEN_ADDRESS from result

# Register EVM token metadata
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN

# Deploy token on Sui with MINT_BURN mode (requires --treasuryCap)
ts-node sui/its register-custom-coin $SYMBOL $NAME $SUI_DECIMALS --salt $SALT --treasuryCap
# Record CHANNEL from result

# Register Sui token metadata
ts-node sui/its register-coin-metadata $SYMBOL

# Link Sui token to EVM token
ts-node sui/its link-coin $SYMBOL $EVM_CHAIN $EVM_TOKEN_ADDRESS \
    --tokenManagerMode mint_burn \
    --destinationTokenManagerMode lock_unlock \
    --channel $CHANNEL \
    --registered
# Record SUI_TOKEN_ID from result

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $SUI_TOKEN_ID $SUI_WALLET_ADDRESS 125 -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $SUI_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS [amount]
```

### Example 3: Link EVM Token (Source - MINT_BURN) with Sui Token (Destination - LOCK_UNLOCK)

Link tokens with different decimals (9 decimals on EVM, 6 decimals on Sui):

```bash
# Common variables
NAME="TEST1234"
SYMBOL="TEST1234"
EVM_DECIMALS=9
SUI_DECIMALS=6
EVM_CHAIN=avalanche
SALT=0x0000000000000000000000000000000000000000000000000000000000000001
EVM_TEMP_SALT="TEST1234"
LOCK_UNLOCK=2
MINT_BURN=4
EVM_WALLET_ADDRESS="0x13f8C723AeB8CA762c652c553a11a11483846d8B"
SUI_WALLET_ADDRESS="0x76f89a9b56dc580aed9f97e2b3bd03d8d24464e38522da9464c15103761c6707"

# Deploy token on Sui
ts-node sui/tokens publish-coin $SYMBOL $NAME $SUI_DECIMALS
# Record SUI_COIN_TYPE from result (without 0x prefix)

# Register Sui token metadata
ts-node sui/its register-coin-metadata $SYMBOL

# Deploy token on EVM
ts-node evm/interchainTokenFactory --action deployInterchainToken \
    --minter $EVM_WALLET_ADDRESS \
    --name $NAME \
    --symbol $SYMBOL \
    --decimals $EVM_DECIMALS \
    --initialSupply 100000000000 \
    --salt $EVM_TEMP_SALT \
    -n $EVM_CHAIN
# Record EVM_TOKEN_ADDRESS from result

# Register EVM token metadata
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN

# Register custom token on EVM with MINT_BURN
ts-node evm/interchainTokenFactory --action registerCustomToken \
    --tokenAddress $EVM_TOKEN_ADDRESS \
    --tokenManagerType $MINT_BURN \
    --operator $EVM_WALLET_ADDRESS \
    --salt $SALT \
    -n $EVM_CHAIN
# Record EVM_TOKEN_ID from result

# Mint tokens to user on Sui
ts-node sui/its-example mint-token $SYMBOL --recipient $SUI_WALLET_ADDRESS

# Give unlinked Sui coin to ITS
ts-node sui/its give-unlinked-coin $SYMBOL $EVM_TOKEN_ID

# Link EVM token to Sui token
ts-node evm/interchainTokenFactory --action linkToken \
    --destinationChain sui \
    --destinationTokenAddress $SUI_COIN_TYPE \
    --tokenManagerType $LOCK_UNLOCK \
    --linkParams "0x" \
    --salt $SALT \
    -n $EVM_CHAIN

# Get token manager address on EVM
ts-node evm/its token-manager-address $EVM_TOKEN_ID -n $EVM_CHAIN
# Record TOKEN_MANAGER_ADDRESS from result

# Transfer mintership to token manager
ts-node evm/its transfer-mintership $EVM_TOKEN_ADDRESS $TOKEN_MANAGER_ADDRESS -n $EVM_CHAIN

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $EVM_TOKEN_ID $SUI_WALLET_ADDRESS 125 -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $EVM_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS [amount]
```

### Example 4: Link EVM Token (Source - LOCK_UNLOCK) with Sui Token (Destination - MINT_BURN)

Link tokens with different decimals (9 decimals on EVM, 6 decimals on Sui):

```bash
# Common variables
NAME="TEST1234"
SYMBOL="TEST1234"
EVM_DECIMALS=9
SUI_DECIMALS=6
EVM_CHAIN=avalanche
SALT=0x0000000000000000000000000000000000000000000000000000000000000001
EVM_TEMP_SALT="TEST1234"
LOCK_UNLOCK=2
MINT_BURN=4
EVM_WALLET_ADDRESS="0x13f8C723AeB8CA762c652c553a11a11483846d8B"
SUI_WALLET_ADDRESS="0x76f89a9b56dc580aed9f97e2b3bd03d8d24464e38522da9464c15103761c6707"

# Deploy token on Sui
ts-node sui/tokens publish-coin $SYMBOL $NAME $SUI_DECIMALS
# Record SUI_COIN_TYPE from result (without 0x prefix)

# Register Sui token metadata
ts-node sui/its register-coin-metadata $SYMBOL

# Deploy token on EVM
ts-node evm/interchainTokenFactory --action deployInterchainToken \
    --minter $EVM_WALLET_ADDRESS \
    --name $NAME \
    --symbol $SYMBOL \
    --decimals $EVM_DECIMALS \
    --initialSupply 100000000000 \
    --salt $EVM_TEMP_SALT \
    -n $EVM_CHAIN
# Record EVM_TOKEN_ADDRESS from result

# Register EVM token metadata
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN

# Register custom token on EVM with LOCK_UNLOCK
ts-node evm/interchainTokenFactory --action registerCustomToken \
    --tokenAddress $EVM_TOKEN_ADDRESS \
    --tokenManagerType $LOCK_UNLOCK \
    --operator $EVM_WALLET_ADDRESS \
    --salt $SALT \
    -n $EVM_CHAIN
# Record EVM_TOKEN_ID from result

# Mint tokens to user on Sui
ts-node sui/its-example mint-token $SYMBOL --recipient $SUI_WALLET_ADDRESS

# Give unlinked Sui coin to ITS with treasuryCapReclaimer flag
ts-node sui/its give-unlinked-coin $SYMBOL $EVM_TOKEN_ID --treasuryCapReclaimer

# Link EVM token to Sui token
ts-node evm/interchainTokenFactory --action linkToken \
    --destinationChain sui \
    --destinationTokenAddress $SUI_COIN_TYPE \
    --tokenManagerType $MINT_BURN \
    --linkParams "0x" \
    --salt $SALT \
    -n $EVM_CHAIN
# Record final EVM_TOKEN_ID from result

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $EVM_TOKEN_ID $SUI_WALLET_ADDRESS 125 -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $EVM_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS [amount]
```

## Troubleshooting & Error Handling

[TODO: Add Sui-specific error handling scenarios, error messages, and solutions]

## Best Practices & Security

1. **Salt Management**: Use unique salts for each token linking operation. On Sui, the salt must be 66 characters (matching Sui address format) and must be unique per token ID being linked
2. **Token Control**: Ensure you have proper control over both tokens (TreasuryCap for MINT_BURN types on Sui)
3. **TreasuryCap Security**: For MINT_BURN token managers on Sui, the TreasuryCap is transferred to the token manager. Ensure this is intended before proceeding
4. **Decimal Precision**: Be aware of decimal differences between chains. ITS Hub automatically handles scaling, but understand the implications for your use case

[TODO: Add operator security and permission management best practices specific to Sui]
