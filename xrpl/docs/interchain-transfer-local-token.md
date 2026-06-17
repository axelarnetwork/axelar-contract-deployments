# XRPL Local Token Interchain Transfer Guide

## Background

This guide validates transfers after an XRPL-local IOU has been linked to an existing EVM token with [link-local-token.md](./link-local-token.md).

It covers both directions:

```text
XRPL IOU -> EVM token
EVM token -> XRPL IOU
```

For `LockUnlock` TokenManagers, transfers require liquidity on both sides:

- XRPL -> EVM releases tokens from the EVM TokenManager.
- EVM -> XRPL releases IOUs from the Axelar XRPL multisig.

## Environment

Use the same variables from the link flow:

```bash
export ENV=testnet
export XRPL_CHAIN=xrpl
export DESTINATION_CHAIN=<evm-amplifier-chain>

export TOKEN_ID=
export TOKEN_ISSUER=
export TOKEN_CURRENCY=
export DESTINATION_TOKEN_ADDRESS=
export TOKEN_MANAGER_ADDRESS=

export XRPL_PRIVATE_KEY=        # XRPL sender/issuer seed for XRPL-origin transfer
export XRPL_RECIPIENT_PRIVATE_KEY=
export XRPL_RECIPIENT=
export EVM_PRIVATE_KEY=
export EVM_RECIPIENT=

export ADMIN_KEY=
export KEYRING_BACKEND=${KEYRING_BACKEND:-test}
export RPC_URL=${RPC_URL:-$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.rpc)")}
export AXELAR_CHAIN_ID=${AXELAR_CHAIN_ID:-$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.chainId)")}
export GAS_PRICES=${GAS_PRICES:-$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.gasPrice)")}
export ARGS=(--from "$ADMIN_KEY" --keyring-backend "$KEYRING_BACKEND" --chain-id "$AXELAR_CHAIN_ID" --gas auto --gas-adjustment 1.5 --gas-prices "$GAS_PRICES" --node "$RPC_URL" -y)
```

## 1. Prepare XRPL Trustlines

The XRPL recipient must trust the IOU issuer.

```bash
npx ts-node xrpl/trust-set.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_RECIPIENT_PRIVATE_KEY" \
  -y \
  "$TOKEN_CURRENCY" \
  "$TOKEN_ISSUER" \
  --limit 1000000000
```

For EVM -> XRPL transfers where the Axelar multisig pays a recipient, the issuer must allow holder-to-holder IOU movement. For new test issuers, set DefaultRipple before creating trustlines:

```bash
npx ts-node xrpl/account-set.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_PRIVATE_KEY" \
  -y \
  --flag 8
```

If trustlines already exist and the issuer side has `NoRipple`, clear it with XRPL tooling before testing EVM -> XRPL. Otherwise the destination XRPL payment can fail with `tecPATH_DRY`.

## 2. Prepare Lock/Unlock Liquidity

For `LockUnlock`, seed the EVM TokenManager with enough destination tokens for XRPL -> EVM transfers.

Use any standard ERC-20 transfer tool. For example, with `cast`:

```bash
export DESTINATION_RPC_URL=$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.chains[process.env.DESTINATION_CHAIN].rpc)")

cast send "$DESTINATION_TOKEN_ADDRESS" \
  "transfer(address,uint256)" \
  "$TOKEN_MANAGER_ADDRESS" \
  "<amount-in-base-units>" \
  --rpc-url "$DESTINATION_RPC_URL" \
  --private-key "$EVM_PRIVATE_KEY"
```

The XRPL multisig also needs IOU balance for EVM -> XRPL transfers. The usual way to seed it is to perform an XRPL -> EVM transfer first.

## 3. XRPL to EVM Transfer

Initiate the transfer with an XRPL `Payment` to the Axelar XRPL multisig.

```bash
npx ts-node xrpl/interchain-transfer.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_PRIVATE_KEY" \
  -y \
  "$TOKEN_CURRENCY.$TOKEN_ISSUER" \
  "<amount>" \
  "$DESTINATION_CHAIN" \
  "$EVM_RECIPIENT" \
  --gasFeeAmount "<drops>"
```

Notes:

- `--gasFeeAmount` is in drops, not XRP.
- For an EVM destination address, pass the normal `0x...` address.

Relay path:

```text
XRPL Payment
-> XrplGateway verify_messages
-> verifier voting / end poll
-> XrplGateway route_incoming_messages
-> AxelarnetGateway execute
-> ITS Hub emits Axelar-to-EVM child message
-> destination MultisigProver construct_proof
-> evm/gateway.js submitProof
-> evm/gateway.js execute
```

If relayers do not complete the destination leg, use the same destination proof and execute pattern from [link-local-token.md](./link-local-token.md).

Verify balances:

```bash
npx ts-node xrpl/balances.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_PRIVATE_KEY"
```

Check the EVM ERC-20 recipient and TokenManager balances with `cast call`, a block explorer, or a small ethers script.

## 4. EVM to XRPL Transfer

Initiate the EVM source transfer. Pass the classic XRPL destination address; `evm/its.js` encodes XRPL destination addresses as ASCII bytes internally.

```bash
npx ts-node evm/its.js interchain-transfer \
  -e "$ENV" \
  -n "$DESTINATION_CHAIN" \
  -p "$EVM_PRIVATE_KEY" \
  -y \
  --destinationChain "$XRPL_CHAIN" \
  --tokenId "0x$TOKEN_ID" \
  --destinationAddress "$XRPL_RECIPIENT" \
  --amount "<amount>"
```

The source EVM transaction emits a message to the ITS Hub. Once the Hub executes it, the Hub emits an Axelar-to-XRPL child message.

If the XRPL destination proof is not handled by relayers, construct it manually on `XrplMultisigProver`. The XRPL prover requires both the child message ID and the child payload.

```bash
export XRPL_MULTISIG_PROVER=$(node -e "const c=require('./axelar-chains-config/info/' + process.env.ENV + '.json'); console.log(c.axelar.contracts.XrplMultisigProver[process.env.XRPL_CHAIN].address)")
export HUB_TO_XRPL_MESSAGE_ID=
export HUB_TO_XRPL_PAYLOAD= # without 0x

MSG=$(jq -nc \
  --arg id "$HUB_TO_XRPL_MESSAGE_ID" \
  --arg payload "$HUB_TO_XRPL_PAYLOAD" \
  '{construct_proof:{cc_id:{source_chain:"axelar",message_id:$id},payload:$payload}}')

axelard tx wasm execute "$XRPL_MULTISIG_PROVER" "$MSG" "${ARGS[@]}"

export XRPL_PROOF_SESSION_ID=
```

When the session is complete, broadcast the signed XRPL `Payment`.

```bash
npx ts-node xrpl/submit-proof.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_PRIVATE_KEY" \
  -y \
  "$XRPL_PROOF_SESSION_ID"
```

Relayers may broadcast the XRPL payment before the manual command. If `submit-proof` returns `tefNO_TICKET`, query the XRPL multisig `account_tx`; the proof may already have succeeded and consumed the ticket.

Verify the XRPL recipient balance:

```bash
npx ts-node xrpl/balances.js \
  -e "$ENV" \
  -n "$XRPL_CHAIN" \
  -p "$XRPL_RECIPIENT_PRIVATE_KEY"
```

## 5. Expected Balance Movement

For `LockUnlock`:

```text
XRPL -> EVM:
  XRPL multisig IOU balance increases.
  EVM TokenManager token balance decreases.
  EVM recipient token balance increases.

EVM -> XRPL:
  EVM TokenManager token balance increases.
  XRPL multisig IOU balance decreases.
  XRPL recipient IOU balance increases.
```

## Troubleshooting

`tecPATH_DRY` on XRPL destination payment:

- Recipient may not have a trustline to `{ issuer, currency }`.
- The issuer side of the trustlines may have `NoRipple` set.
- The XRPL multisig may not hold enough IOU balance.

`tefNO_TICKET` on `xrpl/submit-proof.js`:

- The XRPL multisig ticket was already consumed.
- Check whether a relayer already broadcast the same signed transaction successfully.
- Query XRPL `account_tx` for the multisig before retrying.

No EVM tokens released on XRPL -> EVM:

- The destination TokenManager may not have enough token liquidity for `LockUnlock`.
- The EVM destination proof may be approved but not executed. `submitProof` does not call the destination app; run `evm/gateway.js --action execute` with the Hub-emitted destination payload.

Wrong XRPL destination address encoding:

- For `evm/its.js interchain-transfer` to XRPL, pass the classic XRPL address. Do not pass decoded account bytes.

Wrong XRPL token identity:

- The token is `{ issuer, currency }`, not currency alone.
