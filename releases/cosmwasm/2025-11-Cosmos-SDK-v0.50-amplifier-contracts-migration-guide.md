# Cosmos SDK v0.50 Amplifier Contracts Migration Guide

|                | **Owner**                                                                 |
| -------------- | ------------------------------------------------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io>, @kulikthebird <tomasz@interoplabs.io> |
| **Deployment** |                                                                           |

| **Network**          | **Voting Verifier & Multisig Upgrade** | **Date**   |
| -------------------- | -------------------------------------- | ---------- |
| **Devnet Amplifier** | Completed                              | 2025-11-21 |
| **Stagenet**         | Completed                              | 2025-11-24 |
| **Testnet**          | Completed                              | 2025-12-01 |
| **Mainnet**          | Completed                              | 2025-12-05 |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-11-24 |
| **Stagenet**         | Completed             | 2025-12-03 |
| **Testnet**          | Completed             | 2025-12-03 |
| **Mainnet**          | Completed             | 2025-12-08 |

## Background

This document outlines parameter updates to Amplifier contracts needed to address faster block times resulting from the [Cosmos SDK v0.50 upgrade](../axelard/2025-11-v1.3.0.md).

## Voting Verifier and Multisig migration

Migrate Multisig and all Voting Verifier contracts with updated poll/signing block expiry times. Note that these migrations should be executed _BEFORE_ the Axelard v1.3.0 upgrade.

- Create an `.env` config

```yaml
MNEMONIC=<cosm wasm deployer key mnemonic>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
```

    | Network          | `INIT_ADDRESSES`                                                                                                                                |
    | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
    | devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                                 |
    | stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
    | testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
    | mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |

1. Store Voting Verifier code

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c VotingVerifier \
      -t "Store VotingVerifier contract v2.0.0" \
      -d "Store VotingVerifier contract v2.0.0" \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.0.0
    ```

1. Store and migrate XRPL Voting Verifier code
   Please follow the [XRPL Voting Verifier v2.0.0 release doc](./2025-11-XRPLVotingVerifier-v2.0.0.md)

1. Store Multisig code

    ```bash
        ts-node cosmwasm/submit-proposal.js store \
        -c Multisig \
        -t "Upload Multisig contract v2.4.0" \
        -d "Upload Multisig contract v2.4.0" \
        --instantiateAddresses $INIT_ADDRESSES \
        --version 2.4.0
    ```

1. Update `blockExpiry` on Multisig and all Voting Verifier contracts in chain config

    | Network          | old `blockExpiry` | new `blockExpiry` |
    | ---------------- | ----------------- | ----------------- |
    | devnet-amplifier | 10                | 50                |
    | stagenet         | 10                | 50                |
    | testnet          | 10                | 50                |
    | mainnet          | 10                | 50                |

1. Migrate all VotingVerifier contracts

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts migrate-voting-verifiers --fetchCodeId
    ```

1. Migrate Multisig contract

    ```bash
    ts-node cosmwasm/submit-proposal.js migrate \
    -c Multisig \
    -t "Migrate Multisig to v2.4.0" \
    -d "Multisig to v2.4.0" \
    --msg '{}' \
    --fetchCodeId
    ```

1. Verify Voting Verifier & Multisig contract version

    Once all the store and migration related proposals pass, run this command:

    ```bash
    ts-node cosmwasm/query.ts contract-versions
    ```

    Check the Voting Verifiers and Multisig contracts versions in the `${ENV}.json` file. Voting Verifiers should be upgraded to `v2.0.0` and Multisig to `v2.0.0`.

1. Update block expiry on all Voting Verifier contracts

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts update-voting-verifiers
    ```

1. Update block expiry on Multisig contract

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts update-signing-parameters-for-multisig
    ```

1. Verify updated params

    Wait for all the above proposals to pass and run the following commands:

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts update-voting-verifiers
    ts-node cosmwasm/migrate/sdk50.ts update-signing-parameters-for-multisig
    ```

    Both the above commands should either skip chains that are already updated or print the current value of the `block_expiry`. The parameters should match the expected values.

## Reward pools epoch duration

Update `epoch_duration` on all reward pools. Note that these parameter changes should be executed _AFTER_ the Axelard v1.3.0 upgrade.

| Network              | `epoch_duration` | `rewards_per_epoch` |
| -------------------- | ---------------- | ------------------- |
| **Devnet-amplifier** | `500`            | N/A                 |
| **Stagenet**         | `3000`           | N/A                 |
| **Testnet**          | `3000`           | N/A                 |
| **Mainnet**          | `47250`          | `3424660000`        |

1. Update epoch duration (`devnet-amplifiier`, `stagenet`, `testnet`)

    ```bash
    ts-node cosmwasm/migrate/update-rewards-pool-epoch-duration.ts update --epoch-duration [epoch_duration]
    ```

    For `mainnet`, update epoch duration and rewards per epoch

    ```bash
    ts-node cosmwasm/migrate/update-rewards-pool-epoch-duration.ts update --epoch-duration [epoch_duration] --rewards-per-epoch [rewards_per_epoch]
    ```

1. Verify epoch duration was updated on all pools
    ```bash
    ts-node cosmwasm/migrate/update-rewards-pool-epoch-duration.ts get-reward-pools
    ```
