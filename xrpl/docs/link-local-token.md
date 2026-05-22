# XRPL Local Token Link Guide

## Background

This guide links an XRPL-local IOU to an already deployed token on a remote chain through Axelar ITS.

Use this flow when the XRPL token identity is the source of truth and should determine the ITS token ID:

```text
XRPL token = { issuer: $TOKEN_ISSUER, currency: $TOKEN_CURRENCY }
```

The XRPL currency code alone is not a token. The IOU identity is the issuer plus currency pair.

This flow differs from [link-token.md](./link-token.md), where an EVM token is the source of truth and the XRPL token is a remote representation issued by the Axelar XRPL multisig.

## Prerequisites

- Existing XRPL IOU issuer/currency pair, or a planned issuer/currency pair controlled by the project.
- Existing remote token address on the destination chain.
- Destination chain registered with the ITS Hub.
- Metadata registered for both the XRPL token and the destination token before linking.
- Sufficient permissions for the chosen TokenManager type.

Recommended TokenManager types:

```text
1 = MintBurnFrom
2 = LockUnlock
4 = MintBurn
```

Use `2` (`LockUnlock`) when both sides are existing fixed-supply assets and bridge liquidity is held by the destination TokenManager and XRPL multisig. Use a mint/burn type only when the token permissions support it.

For post-link transfer testing, see [interchain-transfer-local-token.md](./interchain-transfer-local-token.md).

## Environment

Set the working variables:

```bash
export ENV=testnet
export XRPL_CHAIN=xrpl
export DESTINATION_CHAIN=<evm-amplifier-chain>

# Elevated Axelar account for XrplGateway / XrplMultisigProver operations.
export MNEMONIC=

# Optional alternative when using an axelard keyring for manual wasm executes.
export ADMIN_KEY=
export KEYRING_BACKEND=${KEYRING_BACKEND:-test}
export RPC_URL=${RPC_URL:-$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.rpc)")}
export AXELAR_CHAIN_ID=${AXELAR_CHAIN_ID:-$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.chainId)")}
export GAS_PRICES=${GAS_PRICES:-$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.gasPrice)")}
export ARGS=(--from "$ADMIN_KEY" --keyring-backend "$KEYRING_BACKEND" --chain-id "$AXELAR_CHAIN_ID" --gas auto --gas-adjustment 1.5 --gas-prices "$GAS_PRICES" --node "$RPC_URL" -y)

# XRPL seed used only to broadcast XRPL transactions such as the completed multisig TrustSet proof.
export XRPL_PRIVATE_KEY=

# EVM key funded on the destination chain for metadata registration, proof submission, and execute.
export PRIVATE_KEY=

export TOKEN_ISSUER=
export TOKEN_SYMBOL=
export TOKEN_CURRENCY=
export DESTINATION_TOKEN_ADDRESS=
export TOKEN_MANAGER_TYPE=2

# Optional EVM TokenManager operator. Leave empty for no custom operator.
export OPERATOR=
```

Generate a 160-bit XRPL currency code from a symbol:

```bash
npx ts-node xrpl/token.ts token-symbol-to-currency-code "$TOKEN_SYMBOL"
```

If the destination chain should exercise the full Amplifier proof path on testnet, choose a chain with `axelar.contracts.MultisigProver[$DESTINATION_CHAIN]` in the current config.

## 1. Register the XRPL Local Token

Register the issuer/currency pair on `XrplGateway`.

```bash
npx ts-node xrpl/register-local-token.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -m "$MNEMONIC" \
  -y \
  --issuer "$TOKEN_ISSUER" \
  --currency "$TOKEN_CURRENCY"
```

Query the XRPL-derived token ID.

```bash
npx ts-node xrpl/xrpl-token-id.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  --issuer "$TOKEN_ISSUER" \
  --currency "$TOKEN_CURRENCY"

export TOKEN_ID=
```

## 2. Create the XRPL Multisig Trustline

Create a trustline from the Axelar XRPL multisig account to the local IOU. This is an `XrplMultisigProver` operation that constructs a signed XRPL `TrustSet`.

```bash
npx ts-node xrpl/trust-set-multisig.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -m "$MNEMONIC" \
  -y \
  --tokenId "$TOKEN_ID"

export MULTISIG_SESSION_ID=
```

When the multisig session is complete, broadcast the signed XRPL transaction.

```bash
npx ts-node xrpl/submit-proof.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_PRIVATE_KEY" \
  -y \
  "$MULTISIG_SESSION_ID"
```

## 3. Register XRPL Token Metadata

Register XRPL token metadata through `XrplGateway`. This emits a message to the ITS Hub.

```bash
npx ts-node xrpl/register-token-metadata.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -m "$MNEMONIC" \
  -y \
  --issuer "$TOKEN_ISSUER" \
  --currency "$TOKEN_CURRENCY"

export XRPL_METADATA_MESSAGE_ID=
export XRPL_METADATA_PAYLOAD=
```

Route the metadata message through `AxelarnetGateway`.

```bash
export AXELARNET_GATEWAY=$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.contracts.AxelarnetGateway.address)")

axelard tx wasm execute "$AXELARNET_GATEWAY" \
  '{"execute":{"cc_id":{"source_chain":"'"$XRPL_CHAIN"'","message_id":"'"$XRPL_METADATA_MESSAGE_ID"'"},"payload":"'"$XRPL_METADATA_PAYLOAD"'"}}' \
  "${ARGS[@]}"
```

Wait until the ITS Hub stores metadata for the XRPL token address before linking.

## 4. Register Destination Token Metadata

Register the existing destination token metadata from the destination EVM chain.

```bash
npx ts-node evm/its.js register-token-metadata "$DESTINATION_TOKEN_ADDRESS" \
  -e "$ENV" \
  -n "$DESTINATION_CHAIN" \
  -p "$PRIVATE_KEY" \
  -y
```

Wait until this message executes on the ITS Hub.

## 5. Link the XRPL Token ID to the Existing Destination Token

Call `LinkToken` on `XrplGateway`. This sends an ITS Hub `LinkToken` message using the XRPL-derived `TOKEN_ID`.

Without a custom EVM operator:

```bash
npx ts-node xrpl/link-token.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -m "$MNEMONIC" \
  -y \
  --tokenId "$TOKEN_ID" \
  --destinationChain "$DESTINATION_CHAIN" \
  --destinationTokenAddress "$DESTINATION_TOKEN_ADDRESS" \
  --tokenManagerType "$TOKEN_MANAGER_TYPE"
```

With a custom EVM operator:

```bash
npx ts-node xrpl/link-token.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -m "$MNEMONIC" \
  -y \
  --tokenId "$TOKEN_ID" \
  --destinationChain "$DESTINATION_CHAIN" \
  --destinationTokenAddress "$DESTINATION_TOKEN_ADDRESS" \
  --tokenManagerType "$TOKEN_MANAGER_TYPE" \
  --operator "$OPERATOR"
```

Export the emitted values:

```bash
export LINK_MESSAGE_ID=
export LINK_PAYLOAD=
```

Route the link message through the ITS Hub.

```bash
axelard tx wasm execute "$AXELARNET_GATEWAY" \
  '{"execute":{"cc_id":{"source_chain":"'"$XRPL_CHAIN"'","message_id":"'"$LINK_MESSAGE_ID"'"},"payload":"'"$LINK_PAYLOAD"'"}}' \
  "${ARGS[@]}"
```

From the route transaction events, record the Axelar-to-destination message values from the `wasm-contract_called` event:

```bash
export DESTINATION_MESSAGE_ID=
export DESTINATION_PAYLOAD=
export DESTINATION_SOURCE_ADDRESS=
```

## 6. Execute on the Destination EVM Chain

Construct the destination proof on the destination chain's `MultisigProver`.

```bash
export DESTINATION_MULTISIG_PROVER=$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.contracts.MultisigProver[process.env.DESTINATION_CHAIN].address)")

axelard tx wasm execute "$DESTINATION_MULTISIG_PROVER" \
  '{"construct_proof":[{"source_chain":"axelar","message_id":"'"$DESTINATION_MESSAGE_ID"'"}]}' \
  "${ARGS[@]}"

export DESTINATION_SESSION_ID=
```

Submit the proof to the destination EVM Gateway.

```bash
npx ts-node evm/gateway.js \
  -e "$ENV" \
  -n "$DESTINATION_CHAIN" \
  -p "$PRIVATE_KEY" \
  -y \
  --action submitProof \
  --multisigSessionId "$DESTINATION_SESSION_ID"
```

This approves the command in the destination Gateway. Execute the destination ITS call with the payload emitted by the ITS Hub route transaction.

```bash
export DESTINATION_ITS=$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.chains[process.env.DESTINATION_CHAIN].contracts.InterchainTokenService.address)")

npx ts-node evm/gateway.js \
  -e "$ENV" \
  -n "$DESTINATION_CHAIN" \
  -p "$PRIVATE_KEY" \
  -y \
  --action execute \
  --messageId "$DESTINATION_MESSAGE_ID" \
  --sourceChain axelar \
  --sourceAddress "$DESTINATION_SOURCE_ADDRESS" \
  --destination "$DESTINATION_ITS" \
  --payload "$DESTINATION_PAYLOAD"
```

## 7. Verify the Link

Query the destination TokenManager address for `TOKEN_ID`.

```bash
npx ts-node evm/its.js token-manager-address "0x$TOKEN_ID" \
  -e "$ENV" \
  -n "$DESTINATION_CHAIN" \
  -p "$PRIVATE_KEY" \
  -y
```

The destination ITS should now have a TokenManager for `TOKEN_ID` whose registered token is the existing ERC-20 at `DESTINATION_TOKEN_ADDRESS`.

## Notes

- `RegisterLocalToken`, `RegisterTokenMetadata`, and `LinkToken` are privileged `XrplGateway` messages.
- `trust-set-multisig` calls `XrplMultisigProver`, which constructs a signed XRPL `TrustSet`; `submit-proof` broadcasts the transaction to XRPL.
- `LinkToken` requires metadata to be registered for both the XRPL source token address and the destination token address.
- `DeployRemoteToken` deploys a new destination token. `LinkToken` links to an existing destination token.
- `submitProof` on the destination EVM Gateway only approves the command. The destination app call is completed by the following `execute` call into the destination ITS contract.
- This XRPL-origin flow does not use an EVM deployment salt to choose the token ID. The token ID is derived from the XRPL issuer/currency registration and then used as the ID for the remote TokenManager.
