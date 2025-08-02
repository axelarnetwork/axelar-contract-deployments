# XRPL ITS Link Token Guide

## Background

This release provides a generic template for performing custom token linking from an EVM chain to XRPL, using the Axelar Interchain Token Service (ITS).

## Pre-Deployment Setup

Create an `.env` config.

```yaml
PRIVATE_KEY=xyz # set to EVM private key
MNEMONIC=xyz # Axelar Prover Admin account mnemonic
ENV=xyz
```

**_NOTE:_**
The private key variable is permanently associated with the custom token registration and linking. The key must be secured since
it must be reused for future linking of the same token to other chains. The private key is accessed during `evm/*.js` script operations.

| Token Manager Type | Value |
|--------------------|-------|
| `LockUnlock`       | `2`   |
| `MintBurn`         | `4`   |

Choose token manager type based on your token's cross-chain behaviour:
- Use **2 (`LockUnlock`)** if you want the source chain token manager to lock tokens when sending ITS transfers to XRPL and unlock tokens when receiving ITS transfers from XRPL.
- Use **4 (`MintBurn`)** if you want the source chain token manager to burn tokens when sending ITS transfers to XRPL and mint new tokens when receiving ITS transfers from XRPL.

### 1. Token Symbol Pre-Calculation

Before starting the deployment, you need to generate the XRPL Currency Code for your token symbol.

```bash
TOKEN_SYMBOL= # e.g., ABC.xyz
ts-node xrpl/token.ts token-symbol-to-currency-code $TOKEN_SYMBOL
```

### 2. Environment Variables Setup

Set the following environment variables before running the deployment commands. For contract addresses, reference the
`axelar-chains-config/info/<env>.json` file.

```bash
# Contract addresses
XRPL_MULTISIG= # xrpl/contracts/InterchainTokenService/address
XRPL_GATEWAY= # axelar/contracts/XrplGateway/xrpl/address
AXELARNET_GATEWAY= # axelar/contracts/AxelarnetGateway/address
ITS_HUB= # axelar/contracts/InterchainTokenService/address

# Token Details
XRPL_CURRENCY_CODE= # Generated currency from Token Symbol Pre-Calculation
TOKEN_ADDRESS= # Token contract address on the native source chain

# Deployment Parameters
SALT= # Random Salt
TOKEN_MANAGER_TYPE=

OPERATOR="0x" # User specified address or empty bytes
GAS_FEE=  # Estimate using GMP API

SOURCE_CHAIN= # EVM source chain name
DESTINATION_CHAIN= # XRPL destination chain name
```

**API Reference**: Estimate using GMP [API](https://docs.axelarscan.io/gmp#estimateITSFee).

Both `SOURCE_CHAIN` and `DESTINATION_CHAIN` are the case sensitive values from the `axelardId` field in the `axelar-chains-config/info/<env>.json`. E.g., `Ethereum` and `xrpl` on mainnet.

**_NOTE:_**
`axelard` commands require additional parameters for preparing, signing and broadcasting transactions.
Reference guide can be accessed [here](https://docs.axelar.dev/learn/cli/) for supported parameters.

```bash
RPC_URL= # Axelar RPC Node endpoint
AXELAR_CHAIN_ID= # Environment specific Axelar chain id (axelar-dojo-1, axelar-testnet-lisbon-3)
WALLET_NAME= # Wallet name of a funded Axelar account
KEYRING_NAME=
ARGS=(--from $WALLET_NAME --keyring-backend $KEYRING_NAME --chain-id $AXELAR_CHAIN_ID --gas auto --gas-adjustment 1.5 --node $RPC_URL)
```

## Deployment Steps

### 1. Token Metadata Registration on XRPL Gateway

```bash
ts-node xrpl/register-token-metadata.js -n $DESTINATION_CHAIN --issuer $XRPL_MULTISIG --currency $XRPL_CURRENCY_CODE
# Initiated token metadata registration: 69C696A56200BDFB25D7CCB44537239801D69D8B67D8077E2D1012404378A4A0
#
# Message ID: 0x8b49b5ccfb893269a5c263693805874cdeb3c932633ba0301094403c77dad839
#
# Payload: 00000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000f000000000000000000000000000000000000000000000000000000000000004b373336663663373634393533343933393030303030303030303030303030303030303030303030302e724e726a68314b475a6b326a42523377506641516e6f696474464659514b62516e32000000000000000000000000000000000000000000
#
# Token address: 373336663663373634393533343933393030303030303030303030303030303030303030303030302e724e726a68314b475a6b326a42523377506641516e6f696474464659514b62516e32
```

**Extract Values from Command Output:**

```bash
MESSAGE_ID= # message_id
PAYLOAD= # payload
XRPL_TOKEN_ADDRESS='0x' # token_address
```

### 2. Execute Message on the Axelarnet Gateway

```bash
axelard tx wasm execute $AXELARNET_GATEWAY '{"execute": {"cc_id": {"source_chain": "'$DESTINATION_CHAIN'", "message_id": "'$MESSAGE_ID'"}, "payload": "'$PAYLOAD'"}}' "${ARGS[@]}"
```

### 3. Token Metadata Registration on Source Chain

```bash
GAS_FEE=
ts-node evm/its.js -n $SOURCE_CHAIN register-token-metadata $TOKEN_ADDRESS --gasValue $GAS_FEE
```

Wait for GMP transaction to finish executing before proceeding.

### 4. Custom Token Registration on Source Chain

Remove the `--operator` flag to not set any operator.

```bash
ts-node evm/interchainTokenFactory.js -n $SOURCE_CHAIN --action registerCustomToken --tokenAddress $TOKEN_ADDRESS --tokenManagerType $TOKEN_MANAGER_TYPE --operator $OPERATOR --salt $SALT
```

Extract the token ID from the output, without the `0x` prefix.

```bash
TOKEN_ID=
```

### 5. Token Linking

If no operator is defined, set `linkParams` to empty bytes `0x`.
Only the first leg of the remote token deployment (towards the ITS Hub) is required to succeed.
The second leg will fail expectedly.

```bash
GAS_FEE=
ts-node evm/interchainTokenFactory.js -n $SOURCE_CHAIN --action linkToken --destinationChain $DESTINATION_CHAIN --destinationTokenAddress $XRPL_TOKEN_ADDRESS --tokenManagerType $TOKEN_MANAGER_TYPE --linkParams $OPERATOR --salt $SALT --gasValue $GAS_FEE
```

### 6. XRPL Token Instance Registration

**_NOTE:_**
The decimal precision of `15` is hardcoded to avoid double scaling between the XRPL contracts and ITS Hub. Future release of XRPL contracts will use the ITS Hub instance directly.

```bash
ts-node xrpl/register-token-instance.js -n $DESTINATION_CHAIN --tokenId $TOKEN_ID --sourceChain $SOURCE_CHAIN --decimals 15
```

### 7. XRPL Remote Token Registration

```bash
ts-node xrpl/register-remote-token.js -n $DESTINATION_CHAIN --tokenId $TOKEN_ID --currency $XRPL_CURRENCY_CODE
```

## 8. Grant Mint Role to Token Manager

After linking is complete, if you selected the `MintBurn` token manager type, then it is necessary to
grant the token manager minting and burning permissions. Since this logic is token-specific, you'll need to determine the right method to execute for your token.

If your token inherits from the `InterchainToken` contract and uses `MintBurn`, you can run these command to transfer mintership:

```bash
ts-node evm/its.js -n $SOURCE_CHAIN token-manager-address "0x$TOKEN_ID"
```

Extract the token manager address from the command output.

```bash
TOKEN_MANAGER_ADDRESS=
ts-node evm/its.js -n $SOURCE_CHAIN transfer-mintership $TOKEN_ADDRESS $TOKEN_MANAGER_ADDRESS
```

## Cross-Chain Transfer Testing

To test the newly linked token, refer to [2025-02-v.1.0.0.md](../../releases/xrpl/2025-02-v.1.0.0.md).

Ensure that the destination address being used has a trust-line set with the new token. A trust line can be created using the following command, via a funded XRPL account. In the `.env` file, `PRIVATE_KEY` must be set to the seed value of the funded XRPL account.

```bash
node xrpl/trust-set.js -n $DESTINATION_CHAIN $XRPL_CURRENCY_CODE $XRPL_MULTISIG --limit 99999999999999990000000000000000000000000000000000000000000000000000000000000000000000000
```
