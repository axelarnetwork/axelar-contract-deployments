# Cosmwasm Multisig v2.4.4

|                | **Owner**                                |
| -------------- | ---------------------------------------- |
| **Created By** | @themicp <themis@commonprefix.com>       |
| **Deployment** | @themicp <themis@commonprefix.com>       |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Complete              | 2026-11-05 |
| **Stagenet**         | N/A (phased out)      | -          |
| **Testnet**          | Complete              | 2026-11-02 |
| **Mainnet**          | Complete              | 2026-11-02 |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-v2.4.4)

## Background

Security hotfix for the multisig contract.

Changes in this release:

- `RegisterVerifierSet` is restricted to provers authorized on the multisig (any chain) and to the admin or governance
  (`Elevated`). Previously it could be called by any account.

This release is cut from `multisig-v2.4.2`, the version deployed on testnet and mainnet, and contains nothing else.
It deliberately does **not** include the changes on `main` since v2.4.2; in particular the removal of the signature
verifier callback (axelar-amplifier #1141), which the XRPL multisig prover relies on. `multisig-v2.4.3` was built from
`main` and must not be deployed.

No message schema changes. No state migration.

Artifact: `https://static.axelar.network/releases/cosmwasm/multisig/2.4.4/multisig.wasm`,
sha256 `a6967e15b598fc609675014b94d26db4005e452b2d2caece51fb2ac91d12cab7` (signed `checksums.txt` alongside).

Note: `cosmwasm-check` with its default limits rejects this build (a function with 102 locals, the same as v2.4.2).
Axelar raises `max_function_locals` to 2048 on chain (axelar-core v1.5.3+), so check with the chain's limits:

```bash
curl -s $LCD/cosmwasm/wasm/v1/wasm-limits-config | jq -r .config > limits.json
cosmwasm-check --wasm-limits limits.json multisig.wasm
```

## Deployment

- This rollout upgrades the amplifier multisig contract from `v2.4.2` to `v2.4.4`
- There is no state migration involved
- Stagenet is phased out, so this release is not deployed there
- On testnet and mainnet the multisig's wasm admin is the governance module, so the migration is a governance
  proposal there. Only the code upload differs between those two networks.
- On devnet-amplifier the wasm admin is an EOA, so both the upload and the migration are direct transactions.

| Network          | Multisig contract                                                   | Current code      | Upload key                                                             |
| ---------------- | ------------------------------------------------------------------- | ----------------- | ---------------------------------------------------------------------- |
| devnet-amplifier | `axelar19jxy26z0qnnspa45y5nru0l5rmy9d637z5km2ndjxthfxf5qaswst9290r` | code 1645 (2.4.2) | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` (EOA, also wasm admin) |
| testnet          | `axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5` | code 80 (2.4.2)   | `axelar1wz8vvty9g56g43yrzgapv6ue5gsm4ggrxvhww3` (EOA)                  |
| mainnet          | `axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5` | code 58 (2.4.2)   | `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m` (cpi-multisig 3-of-6)  |

### Devnet Amplifier

The `MNEMONIC` must be the wasm admin `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`.

1. Upload new Multisig contract (direct upload, no proposal):

   ```bash
   ts-node cosmwasm/contract.ts store-code -c Multisig -v 2.4.4 -e devnet-amplifier \
     -t "Upload Multisig contract v2.4.4" -d "Upload Multisig contract v2.4.4"
   ```

   Confirm the code hash on chain matches the artifact:

   ```bash
   axelard q wasm code-info $CODE_ID --node $NODE -o json | jq -r '.data_hash // .code_info.data_hash'
   # A6967E15B598FC609675014B94D26DB4005E452B2D2CAECE51FB2AC91D12CAB7
   ```

2. Migrate the Multisig contract directly from the wasm admin:

   ```bash
   ts-node cosmwasm/contract.ts migrate -c Multisig --codeId $CODE_ID --msg '{}' -e devnet-amplifier
   ```

3. Update `devnet-amplifier.json`: `Multisig.codeId`, `Multisig.lastUploadedCodeId`, `Multisig.version` (`2.4.4`),
   `Multisig.storeCodeProposalCodeHash` (`a6967e15…`), and remove the stale `Multisig.storeCodeProposalId`.
   Note the entry was stale before this release (it still described code 1586 / 2.4.0 while code 1645 / 2.4.2 was
   live).

#### Devnet Amplifier record

| Step       | Reference                                                                            |
| ---------- | ------------------------------------------------------------------------------------ |
| Store tx   | `2C8132A9CF0B5007AAEF847905599CDFC892F186CC82E8EB1A4EBBD7255BEEF9` (height 31854732) |
| Code id    | 1686 (`data_hash` `a6967e15…`)                                                       |
| Migrate tx | `22EBF8E9263C9918AB809E4D3216FAC53DCD3D445451A8571D05E184A25F58BB` (height 31854902) |

### Testnet

1. Upload new Multisig contract (direct upload from the upload-allowlisted key, no proposal; the `-t`/`-d` flags are
   required by the CLI but unused for a direct upload):

   ```bash
   ts-node cosmwasm/contract.ts store-code -c Multisig -v 2.4.4 -e testnet \
     -t "Upload Multisig contract v2.4.4" -d "Upload Multisig contract v2.4.4"
   ```

   Confirm the code hash on chain matches the artifact:

   ```bash
   axelard q wasm code-info $CODE_ID --node $NODE -o json | jq -r .data_hash
   # a6967e15b598fc609675014b94d26db4005e452b2d2caece51fb2ac91d12cab7
   ```

2. Upgrade Multisig contract via governance. Use `--codeId` explicitly: a direct upload does not update
   `storeCodeProposalCodeHash`, which `--fetchCodeId` relies on.

   ```bash
   ts-node cosmwasm/contract.ts migrate -c Multisig --codeId $CODE_ID --msg '{}' --governance -e testnet
   ```

#### Testnet record

| Step             | Reference                                                     |
| ---------------- | ------------------------------------------------------------- |
| Code id          | 101 (direct upload, `data_hash` `a6967e15…`)                 |
| Migrate proposal | 670 (passed), executed at height 39167153                     |

### Mainnet

1. Upload new Multisig contract from the `cpi-multisig` 3-of-6, the only non-governance address in mainnet's
   `code_upload_access`: generate the unsigned `MsgStoreCode`, collect three signatures and broadcast. Every signer
   verifies that the bytecode embedded in the unsigned transaction hashes to the release artifact before signing
   (`jq -r '.body.messages[0].wasm_byte_code' unsigned.json | base64 -d | gunzip | shasum -a 256`).

   ```bash
   axelard tx wasm store multisig.wasm \
     --from axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m --chain-id axelar-dojo-1 \
     --gas 25000000 --gas-prices 0.007uaxl \
     --generate-only > unsigned.json
   ```

   After inclusion, read the code id from the `store_code` event and confirm the hash:

   ```bash
   NODE=https://axelar-rpc.polkachu.com:443
   axelard query tx $TXHASH --node $NODE -o json \
     | jq -r '.events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value'
   axelard query wasm code-info $CODE_ID --node $NODE -o json | jq -r .data_hash
   # a6967e15b598fc609675014b94d26db4005e452b2d2caece51fb2ac91d12cab7
   ```

2. Upgrade Multisig contract via governance, submitted from the proposer EOA (`MNEMONIC` in `.env`). Use `--codeId`
   explicitly with the id from step 1 (`--standardProposal` to submit a standard instead of an expedited proposal).

   ```bash
   ts-node cosmwasm/contract.ts migrate -c Multisig --codeId $CODE_ID --msg '{}' --governance -e mainnet
   ```

   Before voting, reviewers confirm the proposal holds a single `MsgMigrateContract` from the gov module for the
   multisig contract above with the expected `code_id` and `msg` `{}`.

3. After the proposal passes, verify (Checklist below) and update `mainnet.json`: `Multisig.codeId`,
   `Multisig.lastUploadedCodeId`, `Multisig.version` (`2.4.4`), `Multisig.storeCodeProposalCodeHash` (`a6967e15…`).
   Note the entry was stale before this release (it still described code 48 / 2.4.0 while code 58 / 2.4.2 was live).

#### Mainnet record

| Step             | Reference |
| ---------------- | --------- |
| Store tx         | `DCBC3B124BE13216A8078067FCA8CFE503F671E04A46D7E376C40B86E6BDDC4A` (height 37129358) |
| Code id          | 69 (`data_hash` `a6967e15…`)                                                        |
| Migrate proposal | 505 (passed), executed at height 37178860                                           |

## Checklist

Verify multisig contract version

```bash
ts-node cosmwasm/query.ts contract-info --contractName Multisig -e $ENV
```

Expected output

```bash
{"contract":"multisig","version":"2.4.4"}
```

Verify state survived the migration: `signing_parameters` unchanged, `authorized_caller` for each chain still returns its
prover, every prover's `current_verifier_set` id is readable via `verifier_set` on the multisig with all keys equal to
their signer addresses, and old signing sessions (including XRPL sessions, which carry `sig_verifier`) load via the
`multisig` query.

Verify the gate:

- `RegisterVerifierSet` from an ordinary account must fail with `sender with role 'NoPrivilege' is not allowed to
  perform this action that requires 'Elevated' permissions`, and the stored set must be unchanged afterwards.
- `RegisterVerifierSet` from the admin with the current set must succeed.
- A prover rotation (`UpdateVerifierSet` / `ConfirmVerifierSet`) still registers its set.

Verify liveness: signing sessions complete after the migration.
