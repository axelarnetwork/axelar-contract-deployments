# Link Token Documentation

## Overview

This document describes the process for linking existing tokens across different chains using the Interchain Token Service (ITS) Hub.

The token linking feature enables:

- Linking existing tokens across connected Amplifier chains via ITS Hub
- Supporting tokens with different decimal precisions through automatic scaling

## How It Works

The token linking process involves two key message types that ITS Hub uses to coordinate the linking:

### Token Metadata Registration

Before linking tokens, ITS Hub needs to know about each token's details (address and decimals) on both chains.

### Token Linking

Once metadata is registered, you can link tokens by specifying which tokens on different chains should be connected.

### Decimal Scaling

ITS Hub automatically handles decimal scaling between linked tokens:

- **Stellar Token**: 7 decimals (1 USDC = 10,000,000 units)
- **EVM Token**: 18 decimals (1 USDC = 1,000,000,000,000,000,000 units)
- **Scaling Factor**: 10^(18-7) = 10^11

When transferring 1 USDC from Stellar to EVM:

- Stellar: 10,000,000 units locked/burned
- EVM: 1,000,000,000,000,000,000 units unlocked/minted

## Link Token Flow

```
1. User controls Token A (Stellar) and Token B (EVM)
2. User → ITS Stellar: registerTokenMetadata(Token A) → ITS Hub
3. User → ITS EVM: registerTokenMetadata(Token B) → ITS Hub
4. User → ITS Stellar: registerCustomToken() → Deploys Token Manager A
5. User → ITS Stellar: linkToken() → ITS Hub
6. ITS Hub: Calculates scaling factor from stored decimals
7. ITS Hub → ITS EVM: Deploy Token Manager B
8. Token linking complete - InterchainTransfer enabled
```

## Prerequisites

Before linking tokens, ensure you have:

1. **Token Control**: You must control both tokens on their respective chains
2. **Token Metadata**: Both tokens must be registered with their metadata
3. **Gas Tokens**: Sufficient gas tokens for cross-chain operations
4. **ITS Deployment**: ITS contracts must be deployed on both chains

## Token Manager Types

The following token manager types are supported:

- `LOCK_UNLOCK` (2): For tokens that are locked on the source chain and unlocked on the destination chain
- `MINT_BURN` (4): For tokens that are burned on the source chain and minted on the destination chain

## Parameters

**Required:**

- `salt`: Unique identifier for the token linking operation
- `tokenAddress`: Address of the token to be linked
- `destinationChain`: Name of the destination chain
- `destinationTokenAddress`: Address of the token on the destination chain
- `type`: Token manager type (LOCK_UNLOCK, MINT_BURN)

**Optional:**

- `--operator`: Operator address for the token manager on the destination chain
- `--gas-token-address`: Gas token address (defaults to XLM on Stellar)
- `--gas-amount`: Gas amount for cross-chain operations

## Step-by-Step Process

### Step 1: Setup Tokens

Setup Token with decimals on both chains:

**Chain A (Stellar):**

```bash
ts-node stellar/its.js deploy-interchain-token <name> <symbol> <decimal> <salt> <initialSupply>
```

**Chain B (EVM):**

```bash
ts-node evm/interchainTokenFactory.js \
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

**Chain A (Stellar):**

```bash
ts-node stellar/its.js register-token-metadata <tokenAddress> --gas-amount <gasAmount>
```

**Chain B (EVM):**

```bash
ts-node evm/its.js --action registerTokenMetadata \
  --tokenAddress <tokenAddress>
  -n <network>
```

### Step 3: Register Custom Token

Register the token on the source chain (Stellar):

```bash
ts-node stellar/its.js register-custom-token <salt> <tokenAddress> <tokenManagerType>
```

### Step 4: Link Token

Link the token to the destination chain:

```bash
ts-node stellar/its.js link-token <salt> <destinationChain> <destinationTokenAddress> <tokenManagerType> \
   --gas-amount <amount> \
   --operator <operator>
```

## Examples

### Example 1: Link USDC Tokens with LOCK_UNLOCK

Link USDC tokens with different decimals (7 decimals on Stellar, 18 decimals on EVM):

```bash
# USDC already exists on both chains

# Register USDC metadata on Stellar (7 decimals)
ts-node stellar/its.js register-token-metadata CB64D3G...USDC --gas-amount 10000000

# Register USDC metadata on EVM (18 decimals)
ts-node evm/its.js --action registerTokenMetadata --tokenAddress 0xa0b86a33...USDC -n evm_chain

# Register custom token on Stellar
ts-node stellar/its.js register-custom-token 0x1234 CB64D3G...USDC LOCK_UNLOCK

# Link token to EVM
ts-node stellar/its.js link-token 0x1234 evm_chain 0xa0b86a33...USDC LOCK_UNLOCK --gas-amount 10000000

# Interchain Token Transfer
ts-node stellar/its.js interchain-transfer <tokenId> evm_chain <destinationAddress> <amount> --gas-amount 10000000
```

### Example 2: Link Custom Token with MINT_BURN

```bash
# Register custom token metadata on both chains
ts-node stellar/its.js register-token-metadata <stellarTokenAddress> --gas-amount 10000000
ts-node evm/its.js --action registerTokenMetadata --tokenAddress <ethereumTokenAddress> -n evm_chain

# Register and link using MINT_BURN type
ts-node stellar/its.js register-custom-token <salt> <stellarTokenAddress> MINT_BURN
ts-node stellar/its.js link-token <salt> evm_chain <ethereumTokenAddress> MINT_BURN --gas-amount 10000000 --operator <operatorAddress>

# Get token Manager on EVM
ts-node evm/its.js --action tokenManagerAddress --tokenId <tokenId> -n evm_chain

# Transfer mintership on EVM
ts-node evm/its.js --action transferMintership --tokenAddress <tokenAddress> --minter <tokenManager> -n evm_chain

# Interchain Token Transfer
ts-node stellar/its.js interchain-transfer <tokenId> evm_chain <destinationAddress> <amount> --gas-amount 10000000
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

1. **Salt Management**: Use unique, cryptographically secure salts for each token linking operation
2. **Gas Estimation**: Always estimate gas costs before executing operations
3. **Token Verification**: Verify token addresses and decimals before linking
4. **Token Control**: Ensure you have proper control over both tokens
5. **Operator Security**: Use secure operator addresses with appropriate permissions
6. **Transaction Verification**: Check gas amount and token permissions if transactions fail
7. **Chain Connectivity**: Ensure both chains are connected to ITS Hub
