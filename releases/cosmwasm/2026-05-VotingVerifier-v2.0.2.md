# VotingVerifier v2.0.2

|                | **Owner**                               |
| -------------- | --------------------------------------- |
| **Created By** | @MakisChristou <makis@commonprefix.com> |
| **Deployment** | @MakisChristou <makis@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v2.0.2)

## Background

Upgrade all `VotingVerifier` contracts from `v2.0.1` → `v2.0.2`. Changes in this release:

- `fix(voting-verifier): cap verify_messages batch at 1000` ([#1167](https://github.com/axelarnetwork/axelar-amplifier/pull/1167)) — caps poll size to bound message-handler gas usage (addresses oversized-poll DoS surfaced in AMPD audit #310).
- `feat: poll by message` ([#1129](https://github.com/axelarnetwork/axelar-amplifier/pull/1129)).
- `feat: allow reevaluation of verify_verifier_set after FailedOnSourceChain` ([#1154](https://github.com/axelarnetwork/axelar-amplifier/pull/1154)).
- `chore: remove voting-verifier and multisig-prover migrations` ([#1133](https://github.com/axelarnetwork/axelar-amplifier/pull/1133)) — migration is a no-op code swap; pass `--msg '{}'`.

There is no state migration; the migrate step just updates the stored code.

## Deployment

This rollout upgrades all `VotingVerifier` contracts from `v2.0.1` to `v2.0.2`. The release uses the multi-chain helper `cosmwasm/migrate/sdk50.ts migrate-voting-verifiers`, which submits one migrate proposal per amplifier chain (skipping any chain whose verifier is already on the new code or whose admin rejects).

> **Note:** This procedure does not cover `XRPLVotingVerifier`. If a new XRPL voting verifier release is required, follow the dedicated XRPL release doc.

1. Create an `.env` config

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

2. Store `VotingVerifier` code

    ```bash
    ts-node cosmwasm/contract.ts store-code \
      -c VotingVerifier \
      -t "Store VotingVerifier contract v2.0.2" \
      -d "Store VotingVerifier contract v2.0.2" \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.0.2 \
      --governance
    ```

    Vote and wait for the proposal to pass before moving on.

3. Migrate all `VotingVerifier` contracts

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts migrate-voting-verifiers --fetchCodeId
    ```

    This iterates every amplifier chain in `${ENV}.json`, submits one migrate proposal per chain (`Migrate Voting Verifier to v2.0.0 for chain <chain>` is the legacy title baked into the helper — the underlying migration still targets the newly-stored code id), and logs/skips on failure.

    Vote on each per-chain migrate proposal.

4. Verify contract versions

    ```bash
    ts-node cosmwasm/query.ts contract-versions
    ```

    All `VotingVerifier[<chain>]` entries should now report `v2.0.2`.

## Rollback

If a chain misbehaves after migration, re-migrate that single chain back to the previous code id:

```bash
ts-node cosmwasm/contract.ts migrate \
  -c VotingVerifier \
  --chainName <chain> \
  --msg '{}' \
  --codeId <previous-code-id> \
  --governance
```
