# VotingVerifier v2.0.2

|                | **Owner**                               |
| -------------- | --------------------------------------- |
| **Created By** | @MakisChristou <makis@commonprefix.com> |
| **Deployment** | @MakisChristou <makis@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | Deployed              | 2026-05-20 |
| **Testnet**          | Deployed              | 2026-05-20 |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v2.0.2)

## Background

Upgrade all `VotingVerifier` contracts from `v2.0.1` → `v2.0.2`. Changes in this release:

- `fix(voting-verifier): cap verify_messages batch at 1000` ([#1167](https://github.com/axelarnetwork/axelar-amplifier/pull/1167)) — caps poll size to bound message-handler gas usage (addresses oversized-poll DoS surfaced in AMPD audit #310).
- `feat: poll by message` ([#1129](https://github.com/axelarnetwork/axelar-amplifier/pull/1129)).
- `feat: allow reevaluation of verify_verifier_set after FailedOnSourceChain` ([#1154](https://github.com/axelarnetwork/axelar-amplifier/pull/1154)).
- `chore: remove voting-verifier and multisig-prover migrations` ([#1133](https://github.com/axelarnetwork/axelar-amplifier/pull/1133)).

The v2.0.2 `MigrateMsg` requires a `chain_codec_address` field (per-chain-type ChainCodec contract address). The batched helper resolves it automatically.

## Deployment

This rollout upgrades all `VotingVerifier` contracts from `v2.0.1` to `v2.0.2` in **two governance proposals total** (one store, one bundled migrate):

1. **Store** — uploads the new WASM (one proposal).
2. **Batched migrate** — bundles `MsgMigrateContract` for every applicable amplifier chain into a single proposal. One vote ratifies them all atomically.

> **Note:** This procedure does not cover `XRPLVotingVerifier`. The helper auto-skips chains whose chain type maps to `XrplVotingVerifier`. If a new XRPL voting verifier release is required, follow the dedicated XRPL release doc.

### 1. Configure environment

```yaml
MNEMONIC=<cosmwasm deployer key mnemonic>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
```

| Network              | `INIT_ADDRESSES`                                                                                                                                |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet Amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                                 |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |

### 2. Store `VotingVerifier` code

```bash
ts-node cosmwasm/contract.ts store-code \
  -c VotingVerifier \
  -t "Store VotingVerifier contract v2.0.2" \
  -d "Store VotingVerifier contract v2.0.2" \
  --instantiateAddresses $INIT_ADDRESSES \
  --version 2.0.2 \
  --governance
```

Vote and wait for the proposal to pass. After it passes, find the new code id:

```bash
curl -s "<LCD>/cosmwasm/wasm/v1/code?pagination.reverse=true&pagination.limit=3" \
  | jq '.code_infos[] | {code_id, creator, data_hash}'
```

Cross-check the top entry's `data_hash` against your local artifact:

```bash
sha256sum artifacts/VotingVerifier-2.0.2.wasm | awk '{print toupper($1)}'
```

That code id is the input to the migrate step. (On testnet, this was `87` from proposal `579`.)

#### Gas / RPC notes

- The 587 KB WASM exceeds the default `cometbft` 1 MB body cap when simulating, which surfaces as `Error: Bad status on response: 413` (the simulate path encodes the tx as hex, ~2× bloat). **Fix:** temporarily set `axelar.gasLimit` in `${ENV}.json` to a fixed number (e.g. `120000000`) — this skips simulate and broadcasts directly (base64, ~1.33×, fits under 1 MB). Revert to `"auto"` after the store-code proposal lands. Storing the wasm uses ~76 M gas (~0.84 AXL at testnet `gasPrice`).
- The qubelabs testnet RPC may still 413 the simulate even with a smaller body. Polkachu (`https://axelar-testnet-rpc.polkachu.com:443`) is a working fallback via `-u`.

### 3. Bundled migration (batched proposal)

```bash
ts-node cosmwasm/migrate/sdk50.ts migrate-voting-verifiers-batch \
  --codeId <NEW_CODE_ID> \
  --newVersion 2.0.2
```

What the helper does:

- Iterates every amplifier chain (`getAmplifierChains`).
- **Skips** chains with no VotingVerifier configured (stale/superseded deployments — e.g. `monad`).
- **Skips** chains whose chain type maps to `XrplVotingVerifier` — those use a separate code and need their own release flow.
- **Skips** chains whose `chainType` has no ChainCodec entry.
- **Auto-skips** chains already on the target `--codeId` (queries each VV's current code id via `client.getContract`).
- Bundles all remaining `MsgMigrateContract` messages into a single `MsgSubmitProposal`. Each message carries `msg: { "chain_codec_address": "<ChainCodec for this chain type>" }`.
- After successful submission, optimistically mutates `${ENV}.json` per migrated chain: `codeId -> <NEW_CODE_ID>`, `version -> "2.0.2"` (when `--newVersion` is passed). `saveConfig` persists on exit.

#### Preview with `--dryRun`

To see what the helper would submit (and what config writes it would perform) without sending a proposal:

```bash
ts-node cosmwasm/migrate/sdk50.ts migrate-voting-verifiers-batch \
  --codeId <NEW_CODE_ID> \
  --newVersion 2.0.2 \
  --dryRun
```

A valid `MNEMONIC` is still required so the client can query current code ids; no transaction is broadcast.

#### Vote

After the helper prints `Proposal submitted: <ID>`, vote on it (single vote ratifies all chains in the bundle):

```bash
cd ~/Repositories/workspace/axe/scripts
./vote_testnet_proposal.sh testnet-nodes <PROPOSAL_ID>
```

> **Atomicity:** Multi-message gov proposals are all-or-nothing. If any `MsgMigrateContract` in the bundle fails at execution, the **whole** proposal fails. The auto-skip on current code id, the `chain_codec_address` per-chain wiring, and the chainType filtering cover the common failure modes.

### 4. Verify

```bash
ts-node cosmwasm/query.ts contract-versions
```

Every `VotingVerifier[<chain>]` should now report `v2.0.2`.

You can also spot-check on-chain code ids per VV:

```bash
for chain in <list-of-chains>; do
  addr=$(jq -r ".axelar.contracts.VotingVerifier[\"$chain\"].address" axelar-chains-config/info/${ENV}.json)
  codeId=$(curl -s "<LCD>/cosmwasm/wasm/v1/contract/$addr" | jq -r '.contract_info.code_id')
  printf "%-22s %s\n" "$chain" "$codeId"
done
```

## Stagenet rollout (2026-05-20)

Proposal 476 — Store VotingVerifier v2.0.2 (code id `98`, hash `F0139170...20310AEF`).

Proposal 477 — Bundled migrate of 10 VotingVerifiers via `migrate-voting-verifiers-batch`: `flow`, `hedera`, `sui`, `xrpl-evm`, `plume`, `hyperliquid`, `monad`, `berachain`, `celo-sepolia`, `solana-stagenet-3`.

Auto-skipped:

- `xrpl` — uses `XrplVotingVerifier`.

## Testnet rollout (2026-05-20)

Proposal 579 — Store VotingVerifier v2.0.2 (code id `87`, hash `F0139170...20310AEF`).

Proposal 585 — Bundled migrate of 11 VotingVerifiers (`flow` and `hedera` were already migrated via the per-chain helper, proposals 581 and 582, before the batched helper was wired up). Final batch: `sui`, `xrpl-evm`, `plume`, `berachain`, `monad-3`, `hyperliquid`, `celo-sepolia`, `memento-demo`, `arc-8`, `stellar-2026-q1-2`, `solana`.

Auto-skipped:

- `monad` — superseded by `monad-3`, no VotingVerifier config.
- `xrpl` — uses `XrplVotingVerifier`.

Not in `chains` (so excluded from `getAmplifierChains`):

- `stellar-2025-q3` — dangling VotingVerifier entry, no `chains` entry.

## Rollback

If a chain misbehaves after migration, re-migrate that single chain back to the previous code id:

```bash
ts-node cosmwasm/contract.ts migrate \
  -c VotingVerifier \
  --chainName <chain> \
  --msg '{"chain_codec_address":"<prior chain codec address>"}' \
  --codeId <previous-code-id> \
  --governance
```

Note: rolling back to a v2.0.1 or earlier code id requires an empty `--msg '{}'` since the `chain_codec_address` field was introduced in v2.0.2's `MigrateMsg`.
