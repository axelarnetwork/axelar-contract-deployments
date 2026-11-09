# Deactivate Blast on Axelar (governance key)

|                | **Owner**                           |
| -------------- | ----------------------------------- |
| **Created By** | @h4writer <hannes@commonprefix.com> |
| **Deployment** | @h4writer <hannes@commonprefix.com> |

| **Network** | **Deployment Status**                                                                                           | **Date**   |
| ----------- | --------------------------------------------------------------------------------------------------------------- | ---------- |
| **Mainnet** | Done ([`1E0DF1C3…`](https://axelarscan.io/tx/1E0DF1C3F2654742B2480B11F2E5FF1E23EBC88B26ADB05F98D405B3899AFA7C)) | 2026-10-28 |

- [Testnet release](./2026-10-Blast-Sepolia-Deactivate-Chain.md): the same flow for `blast-sepolia`, a rehearsal of
  the signing steps

## Background

`axelard tx nexus deactivate-chain blast` deactivates blast in the nexus module. It needs `ROLE_ACCESS_CONTROL`, which
only the governance key holds. It is signed here as a multisig transaction instead of going through a governance
proposal, so it lands in the next block at a time we choose.

The Blast gateway is not upgraded or paused first: it stays on v6.2 (`0xc1712652…`), which has no pause. It keeps
accepting calls after the deactivation, but Axelar no longer relays them.

Once blast is deactivated:

- Messages to blast are no longer delivered. The EVM module marks them failed instead of adding them to a batch
  (`x/evm/abci.go`, `validateMessageForDelivery`).
- Blast gateway events are no longer confirmed, so a `callContract` on blast is never relayed.
- Deactivation does not touch the Blast gateway itself. A batch that was already signed can still be relayed and
  executed there.

Turning blast back on is `activate-chain blast` from the same key, but for an EVM chain that only works once the chain
maintainer threshold is met again (`isActivationThresholdMet`). Treat this as one-way.

| Item              | Value                                                                        |
| ----------------- | ---------------------------------------------------------------------------- |
| Governance key    | `axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx` (4-of-6 `LegacyAminoPubKey`) |
| Account number    | `642856`                                                                     |
| Sequence          | `5` (checked 2026-10-28, re-check before generating the transaction)         |
| Balance           | 1.79 AXL (covers the fee)                                                    |
| Chain ID          | `axelar-dojo-1`                                                              |
| Message           | `/axelar.nexus.v1beta1.DeactivateChainRequest`, `chains: ["blast"]`          |
| Fee               | `1400uaxl` (gas `200000` × `--gas-prices 0.007uaxl`, the minimum)            |
| axelard (mainnet) | `v1.5.5`                                                                     |

Governance key members, in on-chain order. The order matters: the multisig address is derived from it.

| #   | Public key (secp256k1, base64)                 |
| --- | ---------------------------------------------- |
| 1   | `A6rrrrGbNS9/eBkf2a/dQOnIPq7AFCwBIXX5c7fLiYL2` |
| 2   | `AzP5fiv7JptJYgt2L7P7hT16tSVtuyFFxe3JamlB7lWI` |
| 3   | `A5xp7I4vHdP97zPKgcIyo5XKRf4EoTybkOMFoGtPVjwy` |
| 4   | `A3W/yeKZNjzXgQ7rfDd/6STc1YpvL9bVF5zpVTr6pQ+k` |
| 5   | `AvVb/xwFy8f4NxSfT96ReLC++A/3AND+prF4iNQZoThi` |
| 6   | `AlFvtZTcVKcovwRq1UMZkX5mPEI5Cdz1UzYIUcduwbeZ` |

## Timing

Because the gateway can't be paused, anything sent through it after the deactivation is stuck: `callContract`,
`callContractWithToken` and `sendToken` still succeed on blast, the gas is paid and tokens are burned or locked, but the
message is never relayed. So:

- Announce the deactivation date to integrators and dApps on blast in advance, and ask them to stop sending through the
  gateway (and ITS) before it.
- Right before broadcasting, check that no message from or to blast is still in flight on Axelar: messages from blast
  that are not yet confirmed, and messages to blast that are not yet in a signed batch. Both are dropped once blast is
  deactivated. A batch that is already signed can still be relayed afterwards.
- Pick a quiet time and broadcast as soon as the check is clean.

## Steps

### 1. Coordinator: check the key and generate the unsigned transaction

Check that the key still holds the role, and read the current sequence:

```bash
LCD=https://axelar-lcd.qubelabs.io
NODE=https://axelar-rpc.qubelabs.io:443
GOV=axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx

curl -s $LCD/axelar/permission/v1beta1/governance_key | jq -r '.governance_key.public_keys[].key'   # the 6 keys above, same order
curl -s $LCD/cosmos/auth/v1beta1/accounts/$GOV | jq '.account | {account_number, sequence}'
curl -s $LCD/axelar/nexus/v1beta1/chain_state/blast | jq .state.activated                       # true
```

Add the multisig to the local keyring from its public keys (no private keys needed). `--nosort` keeps the on-chain
order; without it the derived address differs.

```bash
i=1
for k in A6rrrrGbNS9/eBkf2a/dQOnIPq7AFCwBIXX5c7fLiYL2 AzP5fiv7JptJYgt2L7P7hT16tSVtuyFFxe3JamlB7lWI \
         A5xp7I4vHdP97zPKgcIyo5XKRf4EoTybkOMFoGtPVjwy A3W/yeKZNjzXgQ7rfDd/6STc1YpvL9bVF5zpVTr6pQ+k \
         AvVb/xwFy8f4NxSfT96ReLC++A/3AND+prF4iNQZoThi AlFvtZTcVKcovwRq1UMZkX5mPEI5Cdz1UzYIUcduwbeZ; do
  axelard keys add gov-member-$i --pubkey "{\"@type\":\"/cosmos.crypto.secp256k1.PubKey\",\"key\":\"$k\"}"
  i=$((i+1))
done
axelard keys add gov-key --multisig gov-member-1,gov-member-2,gov-member-3,gov-member-4,gov-member-5,gov-member-6 \
  --multisig-threshold 4 --nosort
axelard keys show gov-key -a    # must print axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx
```

Generate the unsigned transaction. Gas is fixed because a multisig account can't be simulated with `--gas auto`.

```bash
axelard tx nexus deactivate-chain blast \
  --from $GOV --generate-only \
  --chain-id axelar-dojo-1 --gas 200000 --gas-prices 0.007uaxl \
  --note "Deactivate blast" > blast-deactivate-unsigned.json

jq . blast-deactivate-unsigned.json
sha256sum blast-deactivate-unsigned.json
```

The body must hold exactly one message:

```json
{
    "@type": "/axelar.nexus.v1beta1.DeactivateChainRequest",
    "sender_deprecated": "",
    "chains": ["blast"],
    "sender": "axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx"
}
```

Send `blast-deactivate-unsigned.json`, its sha256, the account number and the sequence to the key holders with the
message below.

### 2. Key holders: sign (4 of 6)

Each signer checks the file and signs it offline with their own key. Replace `<your-key>` with the key's name in your
keyring, and use the sequence from the coordinator's message.

```bash
sha256sum blast-deactivate-unsigned.json    # must match the coordinator's hash
jq '.body.messages, .auth_info.fee' blast-deactivate-unsigned.json

axelard tx sign blast-deactivate-unsigned.json \
  --multisig axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx \
  --from <your-key> \
  --chain-id axelar-dojo-1 --account-number 642856 --sequence 5 \
  --sign-mode amino-json --offline \
  --output-document blast-deactivate-sig-<your-name>.json
```

Return `blast-deactivate-sig-<your-name>.json` to the coordinator.

### 3. Coordinator: combine and broadcast

```bash
axelard tx multisign blast-deactivate-unsigned.json gov-key \
  blast-deactivate-sig-<a>.json blast-deactivate-sig-<b>.json blast-deactivate-sig-<c>.json blast-deactivate-sig-<d>.json \
  --chain-id axelar-dojo-1 --account-number 642856 --sequence 5 --offline \
  --output-document blast-deactivate-signed.json

axelard tx validate-signatures blast-deactivate-signed.json \
  --chain-id axelar-dojo-1 --account-number 642856 --sequence 5 --offline
```

`validate-signatures` must list four valid signatures. Broadcast at the announced time, once the in-flight check
(see [Timing](#timing)) is clean:

```bash
axelard tx broadcast blast-deactivate-signed.json --node $NODE
```

The signatures are bound to sequence `5`. Any other transaction from the governance key before the broadcast
invalidates them, and they must be redone with the new sequence.

### 4. Verify

```bash
axelard q tx <txhash> --node $NODE -o json | jq '.code, [.events[] | select(.type == "chain")]'   # code 0, action deactivated, chain blast
curl -s $LCD/axelar/nexus/v1beta1/chain_state/blast | jq .state.activated                   # false
```

Update the status table at the top, and the Blast row in the
[mainnet release](../evm/2026-10-Consensus-Gateway-Upgrade-v6.5.0-Mainnet.md).

## Message to the key holders

> **Signing request: deactivate blast on Axelar mainnet (governance key)**
>
> We need 4 of the 6 governance key holders to sign a transaction that deactivates the `blast` chain on Axelar mainnet.
> Blast is being wound down, and this transaction stops Axelar from routing messages to and from it. The Blast gateway
> itself is left as it is.
>
> - Signer: governance key `axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx` (4-of-6)
> - Message: `/axelar.nexus.v1beta1.DeactivateChainRequest` with `chains: ["blast"]`, nothing else
> - Fee: 1400uaxl, gas 200000, paid by the governance key
> - Chain ID `axelar-dojo-1`, account number `642856`, sequence `<sequence>`
> - File: `blast-deactivate-unsigned.json`, sha256 `<hash>`
>
> To sign:
>
> ```
> axelard tx sign blast-deactivate-unsigned.json \
>   --multisig axelar1vhn68vmxtkcfmqfcnrfwhfjkm3uj5v09lqpvdx \
>   --from <your-key> \
>   --chain-id axelar-dojo-1 --account-number 642856 --sequence <sequence> \
>   --sign-mode amino-json --offline \
>   --output-document blast-deactivate-sig-<your-name>.json
> ```
>
> Please check the hash and the message before signing, and send the signature file back by <date>. Please don't send
> any other transaction from the governance key until this one is broadcast; it would invalidate the signatures.

## Failure handling

| Situation                                     | Action                                                                                         |
| --------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `keys show gov-key -a` prints another address | The key order is wrong or `--nosort` is missing. Re-add the keys in the on-chain order.        |
| `signature verification failed`               | The sequence or account number changed. Regenerate, re-sign with the current values.           |
| `not authorized to send transaction`          | The governance key changed. Re-check `governance_key` and the derived address.                 |
| `insufficient fees`                           | Raise `--gas-prices` (regenerate and re-sign), or top up the governance key.                   |
| Code 0 but no `chain` / `deactivated` event   | Blast was already deactivated (the handler logs it and returns no error). Check `chain_state`. |
