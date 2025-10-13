# Local ITS Token Deployment Guide

## Background

This release provides a generic template for deploying tokens from XRPL chain to remote EVM chains, using the Axelar Interchain Token Service (ITS).

## Pre-Deployment Setup

Create an `.env` config.

```bash
EVM_PRIVATE_KEY=  # EVM destination chain account private key
XRPL_PRIVATE_KEY= # XRPL account seed
MNEMONIC=         # Axelar Prover Admin account mnemonic
CHAIN=            # XRPL chain name
ENV=xyz           # Amplifier environment name, e.g., mainnet
```

### 1. Environment Variables Setup

Set the following environment variables before running the deployment commands. For contract addresses, reference the
`axelar-chains-config/info/<env>.json` file.

```bash
TOKEN_ISSUER=      # token issuer of local XRPL token, e.g., rMPrLNZt4Zv4eRyN4ew9TRn5iumRG8Htpw
TOKEN_CURRENCY=    # token currency of local XRPL token, e.g., 524C555344000000000000000000000000000000
DESTINATION_CHAIN= # case-sensitive destination chain name, e.g., Ethereum
TOKEN_NAME=        # token name of deployed token on destination chain, e.g., RLUSD
TOKEN_SYMBOL=      # token symbol of deployed token on destination chain, e.g., RLUSD

# Contract addresses
AXELARNET_GATEWAY= # axelar/contracts/AxelarnetGateway/address
DESTINATION_MULTISIG_PROVER= # axelar/contracts/MultisigProver/{DESTINATION_CHAIN}/address
```

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
ts-node xrpl/submit-proof.js -p $XRPL_PRIVATE_KEY $MULTISIG_SESSION_ID
```

### 4. Remote Token Deployment

Deploy XRPL token to a remote destination chain. A new token with the given name and symbol will be deployed on the destination chain.

```bash
ts-node xrpl/deploy-remote-token.js --issuer $TOKEN_ISSUER --currency $TOKEN_CURRENCY --destinationChain $DESTINATION_CHAIN --tokenName $TOKEN_NAME --tokenSymbol $TOKEN_SYMBOL
# Initiated remote token deployment: 3043F52C85E1DF355B890CCC2927BE80D030299AFCDC1CDB2ADE56ABDC79B285

# Message ID: 0x8e610b59f44a44b99e8d70c2089479e973ea66f414094094be769398d0af6305

# Payload: 0000000000000000000000000000000000000000000000000000000000000003000000000000000000000000000000000000000000000000000000000000006000000000000000000000000000000000000000000000000000000000000000a00000000000000000000000000000000000000000000000000000000000000004666c6f770000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001600000000000000000000000000000000000000000000000000000000000000001a7ea6e58bb50cc7f25a9a68a245d5757089b775100509454bc236b56806fc24900000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000f0000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000758595a2e61786c00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000758595a2e61786c000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
```

**Extract Values from Command Output:**

```bash
MESSAGE_ID= # message ID (with 0x prefix), e.g., 0xf7e18e9d76a901ec34313ea60f5e82cf93af25d447ff50d072389b3952b3328f
PAYLOAD= # payload
```

### 5. Execute Message on the Axelarnet Gateway

```bash
axelard tx wasm execute $AXELARNET_GATEWAY '{"execute": {"cc_id": {"source_chain": "'$CHAIN'", "message_id": "'$MESSAGE_ID'"}, "payload": "'$PAYLOAD'"}}' "${ARGS[@]}"
```

**Extract Values from Command Output:**

```bash
MESSAGE_ID= # message ID (axelar -> destination chain, with 0x prefix), e.g., 0x3aadfef6d8e9cfcca7bd8e3e6afd363e26077819e31ce0fda79c56160a2624e7-560430
```

Note that there are multiple message IDs in the events emitted.
This ID pertains to the message from axelar to the destination chain,
with format similar to `0x3aadfef6d8e9cfcca7bd8e3e6afd363e26077819e31ce0fda79c56160a2624e7-560430`.

### 6. Construct Proof on the Destination Chain Multisig Prover

```bash
axelard tx wasm execute $DESTINATION_MULTISIG_PROVER '{"construct_proof":[{"source_chain":"axelar","message_id":"'$MESSAGE_ID'"}]}' "${ARGS[@]}"
```

**Extract Values from Command Output:**

```bash
SESSION_ID= # multisig session ID, e.g., 22306
```

### 7. Submit Proof to Destination Chain

```bash
ts-node evm/gateway.js -p $EVM_PRIVATE_KEY -n $DESTINATION_CHAIN --action submitProof --multisigSessionId $SESSION_ID
```

Note that manual executation of the proof might be necessary if relayer fails to pickup transaction

```bash
ts-node evm/gateway.js -n <destination chain> --action execute --messageId $MESSAGE_ID --sourceChain axelar  --sourceAddress <itsHubAddress>   --destination <destination gateway contract address>   --payload 0x$PAYLOAD
```

Repeat steps 4-7 for every `DESTINATION_CHAIN` that you want the XRPL token to be deployed to.

## Cross-Chain Transfer Testing

To test transferring the newly deployed token, refer to [2025-02-v.1.0.0.md](../../releases/xrpl/2025-02-v.1.0.0.md).
