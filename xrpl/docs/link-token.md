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

### 1. Token Symbol Pre-Calculation

Before starting the deployment, you need to generate the XRPL Currency Code for your token symbol.

```bash
ts-node xrpl/encode-token.js <TOKEN_SYMBOL>
```

### 2. Environment Variables Setup

Set the following environment variables before running the deployment commands:

```bash
# Token Details
XRPL_CURRENCY_CODE="<GENERATED_CURRENCY_CODE>"
XRPL_ISSUER="<XRPL_ISSUER_ADDRESS>"
TOKEN_ADDRESS="<CONTRACT_ADDRESS>"


# Contract Addresses
XRPL_GATEWAY=
AXELARNET_GATEWAY=
ITS_HUB=

# Deployment Parameters
SALT="<RANDOM_SALT>"
TOKEN_MANAGER_TYPE= # Numerical value corresponding to token model (MintBurn, LockUnlock, etc)
OPERATOR= # User specified address
```

**_NOTE:_**
axelard commands require additional parameters for preparing, signing and broadcasting transactions. 
Reference guide can be accessed [here](https://docs.axelar.dev/learn/cli/) for supported parameters.
```bash
# axelard Paramaters
RPC_URL= # Axelar RPC Node endpoint
XRPL_PROVER_ADMIN=[xrpl prover admin key name] # Interactions with XRPL_GATEWAY are permissioned
KEYRING_NAME=[axelard keyring backend name] # Optional depending on where wallet is stored
AXELAR_CHAIN_ID= # Envioronment specific Axelar chain id (axelar-dojo-1, axelar-testnet-lisbon-3)
ARGS=(--from $XRPL_PROVER_ADMIN --keyring-backend $KEYRING_NAME --chain-id $AXELAR_CHAIN_ID --gas auto --gas-adjustment 1.5 --node $RPC_URL)
```

## Deployment Steps

### 1. Token Metadata Registration on XRPL Gateway

```bash
axelard tx wasm execute $XRPL_GATEWAY '{"register_token_metadata": {"xrpl_token": {"issued": {"currency": "'$XRPL_CURRENCY_CODE'", "issuer": "'$XRPL_ISSUER'"}}}}' -o text "${ARGS[@]}"
```

**Extract Values from Command Output:**
```bash
# Extract the following values from the previous transaction results
MESSAGE_ID= #message_id
PAYLOAD= #payload
XRPL_TOKEN_ADDRESS='0x' +  #token_address

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

```bash
ts-node evm/its.js register-token-metadata $TOKEN_ADDRESS --gasValue 1000000000000000000
```

Wait for GMP Transaction to finish executing before proceeding

### 4. Custom Token Registration on Source Chain
**_NOTE:_**
If no operator defined removed --operator flag

```bash
ts-node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress $TOKEN_ADDRESS --tokenManagerType $TOKEN_MANAGER_TYPE --operator $OPERATOR --salt $SALT
```
Note: the GMP transaction is a two step process and only the first leg to the ITS Hub is required to succeed 

From the output set the token Id for subsequent steps
```bash
TOKEN_ID= #tokenID from result without 0x prefix
```

### 5. Token Linking

```bash
ts-node evm/interchainTokenFactory.js --action linkToken --destinationChain xrpl --destinationTokenAddress $XRPL_TOKEN_ADDRESS --tokenManagerType $TOKEN_MANAGER_TYPE --linkParams $OPERATOR --salt $SALT --gasValue 1000000000000000000
```

### 6. XRPL Token Instance Registration

```bash
axelard tx wasm execute $XRPL_GATEWAY '{"register_token_instance": {"token_id": "'$TOKEN_ID'", "chain": "'$CHAIN'", "decimals": 15}}' "${ARGS[@]}"
```
**_NOTE:_**
The decimal precision of `15` is hardcoded to avoid double scaling between the XRPL contracts and ITS Hub. Future release of 
XRPL contracts will use the ITS Hub instance directly.  

### 7. XRPL Remote Token Registration

```bash
axelard tx wasm execute $XRPL_GATEWAY '{"register_remote_token": {"token_id": "'$TOKEN_ID'", "xrpl_currency": "'$XRPL_CURRENCY_CODE'"}}' "${ARGS[@]}"
```

**_NOTE:_**
The following steps depend on the token manager type and underlying source token contract.
If MintBurn model is selected for the token manager, it must be given mint permission by executing the following steps to transfer mintership:

```bash
ts-node evm/its.js token-manager-address "0x$TOKEN_ID"
```
From the output obtain the token manager address for next step

```bash
ts-node evm/its.js transfer-mintership $TOKEN_ADDRESS [token manager address]
```

## Cross-Chain Transfer Testing

To test the connection reference document [2025-02-v.1.0.0.md](./2025-02-v.1.0.0.md).

**_NOTE:_**
Ensure that the destination address being used has a trust-line set with the new currency. This can be performed using the following command using a funded XRPL account:

```bash
node xrpl/trust-set.js -n xrpl $XRPL_CURRENCY_CODE $XRPL_ISSUER --limit 99999999999999990000000000000000000000000000000000000000000000000000000000000000000000000
```
