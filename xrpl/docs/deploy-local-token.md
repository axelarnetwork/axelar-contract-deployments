# Local ITS Token Deployment Guide

## Background

This release provides a generic template for deploying tokens from XRPL chain to remote chains, using the Axelar Interchain Token Service (ITS).

## Pre-Deployment Setup

Create an `.env` config.

```bash
MNEMONIC= # Axelar Prover Admin account mnemonic
CHAIN=    # XRPL chain name
ENV=xyz   # Amplifier environment name, e.g., mainnet
```

### 1. Environment Variables Setup

Set the following environment variables before running the deployment commands.

```bash
TOKEN_ISSUER=      # token issuer of local XRPL token, e.g., rMPrLNZt4Zv4eRyN4ew9TRn5iumRG8Htpw
TOKEN_CURRENCY=    # token currency of local XRPL token, e.g., 524C555344000000000000000000000000000000
DESTINATION_CHAIN= # case-sensitive destination chain name, e.g., Ethereum
TOKEN_NAME=        # token name of deployed token on destination chain, e.g., RLUSD
TOKEN_SYMBOL=      # token symbol of deployed token on destination chain, e.g., RLUSD
```

## Deployment Steps

### 1. XRPL Local Token Registration

Register the local XRPL token on the `XRPLGateway` contract.

```bash
ts-node xrpl/register-local-token.js --issuer $TOKEN_ISSUER --currency $TOKEN_CURRENCY
```

### 2. Query XRPL Token ID

Query the token ID of the newly-registered XRPL token from the `XRPLGateway` contract.

```bash
ts-node xrpl/xrpl-token-id.js --issuer $TOKEN_ISSUER --currency $TOKEN_CURRENCY
```

Set the token ID as an environment variable.

```bash
TOKEN_ID=
```

### 3. Create Trust Line Between Multisig & Token

Create a trust line between the XRPL multisig account and the token, via the `XRPLMultisigProver`.

```bash
ts-node xrpl/trust-set-multisig.js --tokenId $TOKEN_ID
```

Extract the multisig session ID from the command output.

```bash
MULTISIG_SESSION_ID=
ts-node xrpl/trust-set-multisig.js $MULTISIG_SESSION_ID
```

### 4. Remote Token Deployment

Deploy XRPL token to a remote destination chain. A new token with the given name and symbol will be deployed on the destination chain.

```bash
ts-node xrpl/deploy-remote-token.js --issuer $TOKEN_ISSUER --currency $TOKEN_CURRENCY --destinationChain $DESTINATION_CHAIN --tokenName $TOKEN_NAME --tokenSymbol $TOKEN_SYMBOL
```

### 5. Token Instance Registration

Once both legs of the remote token deployment have succeeded, register the token instance
to enable bridging this newly-deployed token between XRPL and the remote destination chain.
Note that XRPL tokens are always deployed to remote chains with `15` decimals of precision.

```bash
ts-node xrpl/register-token-instance.js --tokenId $TOKEN_ID --sourceChain $DESTINATION_CHAIN --decimals 15
```

Repeat steps 4 and 5 for every `DESTINATION_CHAIN` that you want the XRPL token to be deployed to.

## Cross-Chain Transfer Testing

To test transferring the newly deployed token, refer to [2025-02-v.1.0.0.md](../../releases/xrpl/2025-02-v.1.0.0.md).
