# Remote ITS Token Deployment Guide

## Background

This release provides a generic template for deploying tokens from an EVM chain to XRPL, using the Axelar Interchain Token Service (ITS).

## Pre-Deployment Setup

Create an `.env` config.

```bash
PRIVATE_KEY=xyz # EVM source chain account private key
MNEMONIC=       # Axelar Prover Admin account mnemonic
ENV=xyz         # Amplifier environment name, e.g., mainnet
```

### 1. Token Symbol Pre-Calculation

Before starting the deployment, you need to generate the XRPL Currency Code for your token symbol.

```bash
TOKEN_SYMBOL=
ts-node xrpl/token.ts token-symbol-to-currency-code $TOKEN_SYMBOL
```

### 2. Environment Variables Setup

Set the following environment variables before running the deployment commands.

```bash
XRPL_MULTISIG= # xrpl/contracts/InterchainTokenService/address, in the `axelar-chains-config/info/<env>.json` file
XRPL_CURRENCY_CODE= # Generated currency from Token Symbol Pre-Calculation
GAS_FEE=  # Estimate using GMP API for each EVM script
SOURCE_CHAIN= # EVM source chain name
DESTINATION_CHAIN= # XRPL chain name
```

**API Reference**: Estimate using GMP [API](https://docs.axelarscan.io/gmp#estimateITSFee).

Both `SOURCE_CHAIN` and `DESTINATION_CHAIN` are the case sensitive values from the `axelardId` field in the `axelar-chains-config/info/<env>.json`. E.g., `Ethereum` and `xrpl` on mainnet.

## Deployment Steps

### 1. Deploy Remote Token from EVM Chain

Register and deploy a canonical interchain token:

```bash
TOKEN_ADDRESS=  # Token contract address on the native source chain
TOKEN_DECIMALS= # Decimals of the token contract on the native source chain

node evm/interchainTokenFactory.js -n $SOURCE_CHAIN --action registerCanonicalInterchainToken --tokenAddress $TOKEN_ADDRESS

GAS_FEE=
node evm/interchainTokenFactory.js -n $SOURCE_CHAIN --action deployRemoteCanonicalInterchainToken --tokenAddress $TOKEN_ADDRESS --destinationChain $DESTINATION_CHAIN --gasValue $GAS_FEE
```

*Alternatively*, deploy a new interchain token:

```bash
TOKEN_NAME=
TOKEN_SYMBOL=
TOKEN_DECIMALS=
SALT= # Random salt

node evm/interchainTokenFactory.js -n $SOURCE_CHAIN --action deployInterchainToken --name $TOKEN_NAME --symbol $TOKEN_SYMBOL --decimals $TOKEN_DECIMALS --salt $SALT

GAS_FEE=
node evm/interchainTokenFactory.js -n $SOURCE_CHAIN --action deployRemoteInterchainToken --salt $SALT --destinationChain $DESTINATION_CHAIN --gasValue $GAS_FEE
```

Only the first leg of the remote token deployment (towards the ITS Hub) is required to succeed.
The second leg will fail expectedly.

### 2. XRPL Remote Token Registration

Once the first leg of the remote token deployment has succeeded, register the remote token on the `XRPLGateway` contract.

```bash
ts-node xrpl/register-remote-token.js -n $DESTINATION_CHAIN --tokenId $TOKEN_ID --currency $XRPL_CURRENCY_CODE
```

## Cross-Chain Transfer Testing

To test the newly deployed token, refer to [2025-02-v.1.0.0.md](../../releases/xrpl/2025-02-v.1.0.0.md).

Ensure that the destination address being used has a trust-line set with the new token. A trust line can be created using the following command, via a funded XRPL account. In the `.env` file, `PRIVATE_KEY` must be set to the seed value of the funded XRPL account.

```bash
node xrpl/trust-set.js -n xrpl $XRPL_CURRENCY_CODE $XRPL_MULTISIG --limit 99999999999999990000000000000000000000000000000000000000000000000000000000000000000000000
```
