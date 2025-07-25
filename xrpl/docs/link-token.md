# XRPL ITS Link Token Guide

## Background

This release provides a generic template for performing custom token linking from an EVM chain to XRPL using the Axelar Interchain Token Service (ITS). 

## Pre-Deployment Setup

Create an `.env` config. Use `all` for `CHAINS` to run the cmd for every EVM chain, or set a specific chain.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=xyz
```
**_NOTE:_**
The private key variable is permanently associated with the custom token registration and linking. The key must be secured since
it must be reused for future linking of the same token to other chains. The private key is accessed during `evm/*.js` script operations

| Token Manager Type | Value |
|-------------|-------|
| LockUnlock | 2 |
| MintBurn | 4 |

Choose based on your token's cross-chain behaviour:
- Use **2 (LockUnlock)** if you want the source chain token manager to lock tokens when sending ITS transfers to XRPL and unlock tokens when receiving ITS transfers from XRPL.
- Use **4 (MintBurn)** if you want the source chain token manager to burn tokens when sending ITS transfers to XRPL and mint new tokens when receiving ITS transfers from XRPL.

### 1. Token Symbol Pre-Calculation

Before starting the deployment, you need to generate the XRPL Currency Code for your token symbol.

```bash
ts-node xrpl/token.ts token-symbol-to-currency-code <TOKEN_SYMBOL>
```

### 2. Environment Variables Setup
Set the following environment variables before running the deployment commands. For contract addresses reference the 
`axelar-chains-config/info/<env>.json` for needed values.

```bash
# Contract addresses
XRPL_ISSUER= # xrpl/contracts/InterchainTokenService/address
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
GAS_FEE=  # Estimate using gmp api
```
**API Reference:** Estimate using gmp [api](https://docs.axelarscan.io/gmp#estimateITSFee)

**_NOTE:_**
`axelard` commands require additional parameters for preparing, signing and broadcasting transactions. 
Reference guide can be accessed [here](https://docs.axelar.dev/learn/cli/) for supported parameters.
```bash
RPC_URL= # Axelar RPC Node endpoint
AXELAR_CHAIN_ID= # Environment specific Axelar chain id (axelar-dojo-1, axelar-testnet-lisbon-3)
XRPL_PROVER_ADMIN= # Operations against the XRPL_GATEWAY are permissioned and must used the xrpl prover key
KEYRING_NAME=
ARGS=(--from XRPL_PROVER_ADMIN --keyring-backend $KEYRING_NAME --chain-id $AXELAR_CHAIN_ID --gas auto --gas-adjustment 1.5 --node $RPC_URL)
```

## Deployment Steps

### 1. Token Metadata Registration on XRPL Gateway
```bash
axelard tx wasm execute $XRPL_GATEWAY '{"register_token_metadata": {"xrpl_token": {"issued": {"currency": "'$XRPL_CURRENCY_CODE'", "issuer": "'$XRPL_ISSUER'"}}}}' -o text "${ARGS[@]}"
```

**Extract Values from Command Output:**
```bash
MESSAGE_ID= # message_id
PAYLOAD= # payload
XRPL_TOKEN_ADDRESS='0x' +  # token_address

**Extracted Values Example:**
- **Message ID**: `0x8b49b5ccfb893269a5c263693805874cdeb3c932633ba0301094403c77dad839`
- **Token Address**: `373336663663373634393533343933393030303030303030303030303030303030303030303030302e724e726a68314b475a6b326a42523377506641516e6f696474464659514b62516e32`
- **Payload**: `00000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000f000000000000000000000000000000000000000000000000000000000000004b373336663663373634393533343933393030303030303030303030303030303030303030303030302e724e726a68314b475a6b326a42523377506641516e6f696474464659514b62516e32000000000000000000000000000000000000000000`
```

### 2. Execute Message on the Axelarnet Gateway
```bash
axelard tx wasm execute $AXELARNET_GATEWAY '{"execute": {"cc_id": {"source_chain": "xrpl", "message_id": "'$MESSAGE_ID'"}, "payload": "'$PAYLOAD'"}}' "${ARGS[@]}"
```

### 3. Token Metadata Registration on Source Chain
Managed EVM operation using the private key set in .env PRIVATE_KEY variable.
```bash
ts-node evm/its.js register-token-metadata $TOKEN_ADDRESS --gasValue $GAS_FEE
```

Wait for GMP Transaction to finish executing before proceeding

### 4. Custom Token Registration on Source Chain
If no operator defined remove the --operator flag
```bash
ts-node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress $TOKEN_ADDRESS --tokenManagerType $TOKEN_MANAGER_TYPE --operator $OPERATOR --salt $SALT
```

From the output set the token Id without `0x` prefix for subsequent steps
```bash
TOKEN_ID=
```

### 5. Token Linking
If there is no operator defined, set linkParams to empty bytes `0x`
This transaction executes a GMP message where only the first leg to the ITS Hub is required to succeed.

```bash
ts-node evm/interchainTokenFactory.js --action linkToken --destinationChain xrpl --destinationTokenAddress $XRPL_TOKEN_ADDRESS --tokenManagerType $TOKEN_MANAGER_TYPE --linkParams $OPERATOR --salt $SALT --gasValue $GAS_FEE
```

### 6. XRPL Token Instance Registration
CHAIN is the case sensitive value from the axelardId field in the `axelar-chains-config/info/<env>.json` for the source chain where token is originally deployed.

**_NOTE:_**
The decimal precision of `15` is hardcoded to avoid double scaling between the XRPL contracts and ITS Hub. Future release of XRPL contracts will use the ITS Hub instance directly. 

```bash
axelard tx wasm execute $XRPL_GATEWAY '{"register_token_instance": {"token_id": "'$TOKEN_ID'", "chain": "'$CHAIN'", "decimals": 15}}' "${ARGS[@]}"
```


### 7. XRPL Remote Token Registration
```bash
axelard tx wasm execute $XRPL_GATEWAY '{"register_remote_token": {"token_id": "'$TOKEN_ID'", "xrpl_currency": "'$XRPL_CURRENCY_CODE'"}}' "${ARGS[@]}"
```


## 8. Grant mint role to the token manager

After linking is complete and if the MintBurn type is selected for the token manager then it is necessary to
grant the token manager mint permissions. 

## Cross-Chain Transfer Testing

To test the connection reference document [2025-02-v.1.0.0.md](./2025-02-v.1.0.0.md).

Ensure that the destination address being used has a trust-line set with the new currency. This can be performed using the following command using a funded XRPL account. In the .env PRIVATE_KEY must be set to the seed value for a funded XRPL account.

```bash
node xrpl/trust-set.js -n xrpl $XRPL_CURRENCY_CODE $XRPL_ISSUER --limit 99999999999999990000000000000000000000000000000000000000000000000000000000000000000000000
```
