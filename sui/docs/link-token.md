# Link Token Documentation

## Overview

This document explains how to link custom tokens on Sui to other chains using the Interchain Token Service (ITS).

For detailed ITS design specifications and architecture, see **[ARC-1: ITS Hub Multi-Chain Token Linking](https://github.com/axelarnetwork/arcs/blob/main/ARCs/ARC-1.md)**. <!-- skip-check -->

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

**Token Control**

Before linking tokens, ensure you have Token permissions. Permission requirements depend on the manager type:

- **LOCK_UNLOCK**: No token control or minter permissions required for that token
    
- **MINT_BURN**: You must have the [TreasuryCap](https://docs.sui.io/references/framework/sui_sui/coin#sui_coin_TreasuryCap) for the token on Sui to transfer it to the token manager

## Token Manager Types

For Sui, only the following token manager types are supported:

- **LOCK_UNLOCK** (`2_u256`): For tokens that are locked/unlocked on the home chain
- **MINT_BURN** (`4_u256`): For tokens that are burned/minted on the chain

**Important:** Linking two LOCK_UNLOCK tokens is not recommended. One token should be MINT_BURN (requiring `TreasuryCap` transfer if token deployed on Sui) and the other can be LOCK_UNLOCK (no permissions required). Using MINT_BURN on both sides is supported.

## Parameters

**Function Signature**

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

**Requirements**

- `deployer`: An ITS `Channel` of the deployer of the coin as the sender's Sui address will not be tracked, only their `Channel` ID.
- `salt`: Unique identifier for the token linking operation that will be used to generate a unique `tokenId`. On Sui, the `salt` must be 64 characters (e.g. 32 bytes), matching the Sui address format.
- `destination_chain`: Name of the destination chain.
- `destination_token_address`: Address of the token on the destination chain.
- `token_manager_type`: Token manager type on Sui (e.g., `2_u256` for LOCK_UNLOCK or, `4_u256` for MINT_BURN)

**Optional**
- `link_params`: Bytes representation of an address on the destination chain that will be `Operator` of the destination token. Or, use an empty `vector` for no `Operator`.

## Channel Role & Security

#### `Channel` & Sender Identity

Sui uses an object ownership model for security. Contracts cannot access sender parameters (such as the sender's Sui address), but capabilities can be granted by owning or transferring an object that posses the permissions required.

In the context of ITS, the `Channel` object is used in place of the sender's Sui address managing, deploying and transferring tokens.

#### Notable Entities Using `Channel`

1. **Operator:** controls operational settings for a token manager, including flow limits and pausing.
2. **Distributor:** has minting and burning privileges for tokens with MINT_BURN token manager types.
3. **Deployer:** the original entity that registered or deployed a token. During token linking, deployers can auto-enable operators on the destination chain, and during custom token registration their channel is used for `TokenId` derivation.

#### Role of `Channel` in `TokenId` Derivation

**Custom Token ID Derivation**

When registering a custom token for linking, the token ID is derived from three components:

1. **Chain Name Hash** (`Bytes32`): Unique identifier for the Sui chain instance
2. **Deployer** (`Channel`): The address of the deployer's Channel object
3. **Salt** (`Bytes32`): A user-provided unique value (must be 0x + 64 hex characters on Sui)

Derivation Formula:

```move
let token_id = hash(PREFIX_CUSTOM_TOKEN_ID, chain_name_hash, deployer.to_address(), salt)
```

**Note:** the same `Channel` cannot re-use the same `salt`.

#### Role of `Channel` in `link_coin`

During token linking the deployer is verified as the creator of the token using the provided `Channel` and `salt` by deriving the token id.

Derivation Formula:

```move
let token_id = token_id::custom_token_id(&self.chain_name_hash, deployer, &salt);
```

## Operator Role & Security

**Operator Role:**

When Sui [receives](https://github.com/axelarnetwork/axelar-cgp-sui/blob/40458a1d6577f97416522f17e529a3a7fcd8f5c6/move/interchain_token_service/sources/interchain_token_service.move#L269-L273) a link message, if a `Channel` is included in the `link_params`, it will be automatically added as an Operator of the Sui coin.<!-- skip-check -->

When sui [creates](https://github.com/axelarnetwork/axelar-cgp-sui/blob/40458a1d6577f97416522f17e529a3a7fcd8f5c6/move/interchain_token_service/sources/interchain_token_service.move#L172-L184) a link message, if an address is included in the `link_params`, it will be added as an Operator on the destination chain (actual behaviour depends on destination implementation).<!-- skip-check -->

**Security:** 

The operator cannot steal tokens directly, but can modify settings that affect the Interchain Token Service. Use trusted channels and addresses only.

**Note:** The deployer account (caller of `linkToken`) must also be secure, as it has the authority to initiate token linking operations and Operators.

## Step-by-Step Process

**Example Configuration**

- Chain A (Sui): Source chain using *LOCK_UNLOCK* token manager type
- Chain B (EVM): Destination chain using *MINT_BURN* token manager type

### Step 1: Setup Tokens

Deploy test tokens on both chains:

**Chain A (Sui)**

```bash
# Deploy token on Sui
ts-node sui/tokens publish-coin <symbol> <name> <decimals>
```

**Chain B (EVM)**

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

**Chain A (Sui)**

```bash
ts-node sui/its register-coin-metadata <symbol>
```

**Chain B (EVM)**

```bash
ts-node evm/its register-token-metadata <tokenAddress> -n <network>
```

### Step 3: Register Custom Token

Register the token on the source chain (Sui):

```bash
# For LOCK_UNLOCK token manager
ts-node sui/its register-custom-coin <symbol> <name> <decimals> --salt <salt> --channel <channel>

# For MINT_BURN token manager (requires --treasuryCap flag)
ts-node sui/its register-custom-coin <SYMBOL> <NAME> <DECIMALS> --salt <SALT> --treasuryCap --channel <channel>
```

**Notes:** 
1. If the `--treasuryCap` flag is passed, the coin's `TreasuryCap` is automatically transferred to the Sui ITS contract. For `MINT_BURN` token managers, transferring the `TreasuryCap` to the ITS contract is required.
2. If the `--channel <channel>` flag is not used a channel will be automatically created and transferred to the address of the command caller. 

### Step 4: Link Token

Link the token to the destination chain:

```bash
ts-node sui/its link-coin <symbol> <destination-chain> <destination-token-address> \
    --tokenManagerMode <lock_unlock|mint_burn> \
    --destinationTokenManagerMode <lock_unlock|mint_burn> \
    --channel <channel> \
    --registered

# Record the Token ID from the result.
TOKEN_ID=<from-result>
```

**Note:** `link-coin` must use the same `Channel` that was used to register the custom token (e.g. in the previous step).

**On EVM (if destination uses _MINT_BURN_)**

```bash
# Get token manager address on the destination chain
ts-node evm/its token-manager-address <tokenId> -n <network>

# Transfer mintership to the token manager
ts-node evm/its transfer-mintership <tokenAddress> <tokenManagerAddress> -n <network>
```

## Examples

Basic configuration for all examples:

```bash
# Common variables
NAME="Test Link Coin"
SYMBOL="TEST"
EVM_DECIMALS=9
SUI_DECIMALS=6
EVM_CHAIN=avalanche
SALT=0x0000000000000000000000000000000000000000000000000000000000000001
EVM_TEMP_SALT="TEST1234"
EVM_WALLET_ADDRESS="0x13f8C723AeB8CA762c652c553a11a11483846d8B"
SUI_WALLET_ADDRESS="0x76f89a9b56dc580aed9f97e2b3bd03d8d24464e38522da9464c15103761c6707"
CHANNEL="0x028680c11ddb66705c1609d204b108737003d140d27e9096fe72b6bc2dadfeeb"
TRANSFER_AMOUNT=1
LOCK_UNLOCK=2
MINT_BURN=4
```

### Example 1: Link Token on Sui (Source - _LOCK_UNLOCK_) with EVM Token (Destination - _MINT_BURN_)

```bash
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
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"

# Register EVM token metadata
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN

# Deploy and register token on Sui with LOCK_UNLOCK mode
ts-node sui/its register-custom-coin $SYMBOL $NAME $SUI_DECIMALS --salt $SALT --channel $CHANNEL

# Register Sui token metadata
ts-node sui/its register-coin-metadata $SYMBOL

# Link Sui token to EVM token
ts-node sui/its link-coin $SYMBOL $EVM_CHAIN $EVM_TOKEN_ADDRESS \
    --tokenManagerMode lock_unlock \
    --destinationTokenManagerMode mint_burn \
    --channel $CHANNEL \
    --registered

# Record SUI_TOKEN_ID from result
SUI_TOKEN_ID="TOKEN_ID"

# Get token manager address on EVM
ts-node evm/its token-manager-address $SUI_TOKEN_ID -n $EVM_CHAIN

# Record TOKEN_MANAGER_ADDRESS from result
TOKEN_MANAGER_ADDRESS="EVM_TOKEN_MANAGER"

# Transfer mintership to token manager
ts-node evm/its transfer-mintership $EVM_TOKEN_ADDRESS $TOKEN_MANAGER_ADDRESS -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $SUI_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS $TRANSFER_AMOUNT

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $SUI_TOKEN_ID $SUI_WALLET_ADDRESS $TRANSFER_AMOUNT -n $EVM_CHAIN
```

### Example 2: Link Token on Sui (Source - _MINT_BURN_) with EVM Token (Destination - _LOCK_UNLOCK_)

```bash
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
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"

# Register EVM token metadata
ts-node evm/its register-token-metadata $EVM_TOKEN_ADDRESS -n $EVM_CHAIN

# Deploy token on Sui with MINT_BURN mode (requires --treasuryCap)
ts-node sui/its register-custom-coin $SYMBOL $NAME $SUI_DECIMALS --salt $SALT --treasuryCap --channel $CHANNEL

# Register Sui token metadata
ts-node sui/its register-coin-metadata $SYMBOL

# Link Sui token to EVM token
ts-node sui/its link-coin $SYMBOL $EVM_CHAIN $EVM_TOKEN_ADDRESS \
    --tokenManagerMode mint_burn \
    --destinationTokenManagerMode lock_unlock \
    --channel $CHANNEL \
    --registered

# Record SUI_TOKEN_ID from result
SUI_TOKEN_ID="TOKEN_ID"

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $SUI_TOKEN_ID $SUI_WALLET_ADDRESS $TRANSFER_AMOUNT -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $SUI_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS $TRANSFER_AMOUNT
```

### Example 3: Link EVM Token (Source - _MINT_BURN_) with Sui Token (Destination - _LOCK_UNLOCK_)

```bash
# Deploy token on Sui
ts-node sui/tokens publish-coin $SYMBOL $NAME $SUI_DECIMALS

# Record SUI_COIN_TYPE from result (without 0x prefix)
# Sui coin types (`CoinType`) have the following format: `$PACKAGE_ID::$MODULE_NAME::$COIN_SYMBOL`
# Example Sui `CoinType`: 0x265ce251c3a65f0ddfe0d90a62b758662209813b26adb7b76f260c148bc92350::test::TEST
SUI_COIN_TYPE="package::module::symbol"

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
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"

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
EVM_TOKEN_ID="TOKEN_ID"

# Give unlinked Sui coin to ITS 
# Note: you may wish to mint coins before giving ITS the TreasuryCap
# (E.g.: `ts-node sui/its-example mint-token $SYMBOL --recipient $SUI_WALLET_ADDRESS`)
ts-node sui/its give-unlinked-coin $SYMBOL $EVM_TOKEN_ID --treasuryCapReclaimer

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
TOKEN_MANAGER_ADDRESS="EVM_TOKEN_MANAGER"

# Transfer mintership to token manager
ts-node evm/its transfer-mintership $EVM_TOKEN_ADDRESS $TOKEN_MANAGER_ADDRESS -n $EVM_CHAIN

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $EVM_TOKEN_ID $SUI_WALLET_ADDRESS $TRANSFER_AMOUNT -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $EVM_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS $TRANSFER_AMOUNT
```

### Example 4: Link EVM Token (Source - _LOCK_UNLOCK_) with Sui Token (Destination - _MINT_BURN_)

```bash
# Deploy token on Sui
ts-node sui/tokens publish-coin $SYMBOL $NAME $SUI_DECIMALS

# Record SUI_COIN_TYPE from result (without 0x prefix)
# Sui coin types (`CoinType`) have the following format: `$PACKAGE_ID::$MODULE_NAME::$COIN_SYMBOL`
# Example Sui `CoinType`: 0x265ce251c3a65f0ddfe0d90a62b758662209813b26adb7b76f260c148bc92350::test::TEST
SUI_COIN_TYPE="package::module::symbol"

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
EVM_TOKEN_ADDRESS="EVM_TOKEN_ADDRESS"

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
EVM_TOKEN_ID="TOKEN_ID"

# Give unlinked Sui coin to ITS 
# Note: you may wish to mint coins before giving ITS the TreasuryCap
# (E.g.: `ts-node sui/its-example mint-token $SYMBOL --recipient $SUI_WALLET_ADDRESS`)
ts-node sui/its give-unlinked-coin $SYMBOL $EVM_TOKEN_ID --treasuryCapReclaimer

# Link EVM token to Sui token
ts-node evm/interchainTokenFactory --action linkToken \
    --destinationChain sui \
    --destinationTokenAddress $SUI_COIN_TYPE \
    --tokenManagerType $MINT_BURN \
    --linkParams "0x" \
    --salt $SALT \
    -n $EVM_CHAIN

# Record EVM_TOKEN_ID from result
EVM_TOKEN_ID="TOKEN_ID"

# Test interchain transfer from EVM to Sui
ts-node evm/its interchain-transfer sui $EVM_TOKEN_ID $SUI_WALLET_ADDRESS $TRANSFER_AMOUNT -n $EVM_CHAIN

# Test interchain transfer from Sui to EVM
ts-node sui/its interchain-transfer $EVM_TOKEN_ID $EVM_CHAIN $EVM_WALLET_ADDRESS $TRANSFER_AMOUNT
```

## Troubleshooting & Error Handling

Error message types in Sui begin with an uppercase E, followed by the error name in ucfirst format.

**Untrusted Chain:**
```
EUntrustedChain: the chain is not trusted
```

Problem: attempted to create an interchain message (`prepare_hub_message`) for an unapproved chain

Solution: use the admin acount (holding the `OwnerCap`) to add the target destination chain (`add_trusted_chains`), or, use an already approved chain for the destination chain.

**Destination Token Address Missing**
```
EEmptyTokenAddress: cannot deploy a remote custom token to an empty token address
```

Problem: `link_coin` was called without specifying a valid remote token address.

Solution: ensure the correct remote token address is specified when calling `link_coin`

**Unsupported Native Manager Type**
```
ECannotDeployInterchainTokenManager: cannot deploy an interchain token token manager type remotely
```

Problem: calling `link_coin` with an interchain token native token manager type, for the destination chain, is unsupported and coin linking fails.

Solution: use MINT_BURN or LOCK_UNLOCK token manager type for the destination token manager parameter of `link_coin`.

**Invalid Remote Destination**
```
ECannotDeployRemotelyToSelf: cannot deploy custom token to this chain remotely, use register_custom_coin instead
```

Problem: Sui has been declared as the remote destination in a call to `link_coin`.

Solution: use a different chain name (not Sui) for the `destination` parameter.

**CoinType Mismatch**
```
ECoinTypeMissmatch: receiving coin type does not match the coin type specified
```

Problem: the `receive_link_coin<T>` type argmument generic (`T`) does not match the coin type that was decoded from GMP as the destination token address.

Solution: Ensure the correct `CoinType` is being broadcast in the `linkToken` request from the source chain.

## Best Practices & Security

1. **Salt Management**: Use unique salts for each token linking operation. On Sui, the salt must be 0x + 64 hex characters (e.g. 32 bytes) matching Sui address format.
2. **Token Control**: Ensure you have proper control over both tokens (e.g. `TreasuryCap` for MINT_BURN token manager types on Sui).
3. **TreasuryCap Security**: For MINT_BURN token managers, the `TreasuryCap` is transferred to the token manager. Ensure this is intended before proceeding, and that any precedent transactions (such as minting yourself tokens) has been taken care of before transferring the `TreasuryCap`.
4. **Decimal Precision**: Be aware of decimal differences between chains. ITS Hub automatically handles scaling, but understand the implications for your use case.
