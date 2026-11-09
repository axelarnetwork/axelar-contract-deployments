# Deactivate Blast Sepolia on Axelar testnet (governance key)

|                | **Owner**                           |
| -------------- | ----------------------------------- |
| **Created By** | @h4writer <hannes@commonprefix.com> |
| **Deployment** | @h4writer <hannes@commonprefix.com> |

| **Network** | **Deployment Status**                                                                                                   | **Date**   |
| ----------- | ----------------------------------------------------------------------------------------------------------------------- | ---------- |
| **Testnet** | Done ([`096860F7…`](https://testnet.axelarscan.io/tx/096860F7710A59809F6CD085F834BB2EA64A50A3ED2A097AA9F3C56047072E87)) | 2026-10-28 |

- [Mainnet release](./2026-10-Blast-Deactivate-Chain.md): the same steps for `blast` on mainnet

## Background

The testnet copy of the [mainnet release](./2026-10-Blast-Deactivate-Chain.md):
`axelard tx nexus deactivate-chain blast-sepolia`, signed by the testnet governance key as a multisig transaction.

**`blast-sepolia` is already deactivated on testnet** (`activated: false`, checked 2026-10-28). The deactivate handler
then only logs `chain blast-sepolia already deactivated` and returns without an error, so this transaction succeeds
and changes nothing. That makes it a safe rehearsal of the mainnet signing flow (`keys add --multisig --nosort`,
`tx sign --multisig`, `multisign`, `validate-signatures`, `broadcast`), which has not been run end to end yet.

A simulation against testnet on 2026-10-28 (dummy signature, not broadcast) passed the role check and executed the
message with 54,685 gas, without a `chain` event. The real transaction used 58,194 gas. The same simulation for an active chain emits
`chain action=deactivated`.

Differences from mainnet:

- The governance key is a **1-of-1** multisig, so one signature is enough.
- The key that signs is the member key; its own address (`axelar169hh35659g3xdf8724pw4vhwhwx25h088avxde`) does
  **not** hold the role. Sign with `--multisig` so the transaction comes from the multisig account.

| Item              | Value                                                                        |
| ----------------- | ---------------------------------------------------------------------------- |
| Governance key    | `axelar1vzs6rll05ecdymtmr48n9a89q5v2sph268feng` (1-of-1 `LegacyAminoPubKey`) |
| Member address    | `axelar169hh35659g3xdf8724pw4vhwhwx25h088avxde` (no role, signs only)        |
| Account number    | `7279148`                                                                    |
| Sequence          | `5` (checked 2026-10-28, re-check before generating the transaction)         |
| Balance           | 4.76 AXL (covers the fee)                                                    |
| Chain ID          | `axelar-testnet-lisbon-3`                                                    |
| Message           | `/axelar.nexus.v1beta1.DeactivateChainRequest`, `chains: ["blast-sepolia"]`  |
| Fee               | `1400uaxl` (gas `200000` × `--gas-prices 0.007uaxl`, the minimum)            |
| axelard (testnet) | `v1.5.3`                                                                     |

Governance key member:

| #   | Public key (secp256k1, base64)                 |
| --- | ---------------------------------------------- |
| 1   | `AoPLcobioMvyNW7qiZIyybc3aM+BVV2EFnjp/avIjp5c` |

## Steps

### 1. Coordinator: check the key and generate the unsigned transaction

```bash
LCD=https://axelar-testnet-lcd.qubelabs.io
NODE=https://axelar-testnet-rpc.qubelabs.io:443
GOV=axelar1vzs6rll05ecdymtmr48n9a89q5v2sph268feng

curl -s $LCD/axelar/permission/v1beta1/governance_key | jq -r '.governance_key.public_keys[].key'   # the key above
curl -s $LCD/cosmos/auth/v1beta1/accounts/$GOV | jq '.account | {account_number, sequence}'
curl -s $LCD/axelar/nexus/v1beta1/chain_state/blast-sepolia | jq .state.activated               # false (already deactivated)
```

Add the multisig to the local keyring from its public key (no private key needed):

```bash
axelard keys add gov-testnet-member-1 \
  --pubkey '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"AoPLcobioMvyNW7qiZIyybc3aM+BVV2EFnjp/avIjp5c"}'
axelard keys add gov-testnet-key --multisig gov-testnet-member-1 --multisig-threshold 1 --nosort
axelard keys show gov-testnet-key -a    # must print axelar1vzs6rll05ecdymtmr48n9a89q5v2sph268feng
```

If the signer runs this on the same machine and already has the member's private key in the keyring, skip
`gov-testnet-member-1` and build `gov-testnet-key` from that key's name instead.

Generate the unsigned transaction:

```bash
axelard tx nexus deactivate-chain blast-sepolia \
  --from $GOV --generate-only \
  --chain-id axelar-testnet-lisbon-3 --gas 200000 --gas-prices 0.007uaxl \
  --note "Deactivate blast-sepolia" > blast-sepolia-deactivate-unsigned.json

jq . blast-sepolia-deactivate-unsigned.json
sha256sum blast-sepolia-deactivate-unsigned.json
```

The body must hold exactly one message:

```json
{
    "@type": "/axelar.nexus.v1beta1.DeactivateChainRequest",
    "sender_deprecated": "",
    "chains": ["blast-sepolia"],
    "sender": "axelar1vzs6rll05ecdymtmr48n9a89q5v2sph268feng"
}
```

### 2. Key holder: sign

```bash
sha256sum blast-sepolia-deactivate-unsigned.json    # must match the coordinator's hash
jq '.body.messages, .auth_info.fee' blast-sepolia-deactivate-unsigned.json

axelard tx sign blast-sepolia-deactivate-unsigned.json \
  --multisig axelar1vzs6rll05ecdymtmr48n9a89q5v2sph268feng \
  --from <member-key> \
  --chain-id axelar-testnet-lisbon-3 --account-number 7279148 --sequence 5 \
  --sign-mode amino-json --offline \
  --output-document blast-sepolia-deactivate-sig.json
```

`<member-key>` is the keyring name of the key with public key `AoPLcobi…`
(address `axelar169hh35659g3xdf8724pw4vhwhwx25h088avxde`).

### 3. Coordinator: combine and broadcast

```bash
axelard tx multisign blast-sepolia-deactivate-unsigned.json gov-testnet-key blast-sepolia-deactivate-sig.json \
  --chain-id axelar-testnet-lisbon-3 --account-number 7279148 --sequence 5 --offline \
  --output-document blast-sepolia-deactivate-signed.json

axelard tx validate-signatures blast-sepolia-deactivate-signed.json \
  --chain-id axelar-testnet-lisbon-3 --account-number 7279148 --sequence 5 --offline
axelard tx broadcast blast-sepolia-deactivate-signed.json --node $NODE
```

`validate-signatures` must list one valid signature. The signature is bound to sequence `5`; any other transaction from
the governance key before the broadcast invalidates it.

### 4. Verify

```bash
axelard q tx <txhash> --node $NODE -o json | jq '.code, [.events[] | select(.type == "chain")]'   # code 0, [] (already deactivated)
curl -s $LCD/axelar/nexus/v1beta1/chain_state/blast-sepolia | jq .state.activated                 # false
curl -s $LCD/cosmos/auth/v1beta1/accounts/$GOV | jq .account.sequence                              # 6
```

Code 0 and the sequence moving to 6 show the multisig flow works. Note anything that differed from the commands above
in the [mainnet release](./2026-10-Blast-Deactivate-Chain.md) before running it there.

## Failure handling

| Situation                                             | Action                                                                              |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------- |
| `keys show gov-testnet-key -a` prints another address | The multisig was built from the wrong key or without `--nosort`. Re-add it.         |
| `signature verification failed`                       | The sequence or account number changed. Regenerate and re-sign with current values. |
| `not authorized to send transaction`                  | Signed as the member address instead of the multisig. Sign with `--multisig $GOV`.  |
| `insufficient fees`                                   | Raise `--gas-prices` (regenerate and re-sign), or top up the governance key.        |
