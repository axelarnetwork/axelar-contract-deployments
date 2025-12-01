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
| **Mainnet**          | -                                      | TBD        |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-11-24 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

## Background

This document outlines parameter updates to Amplifier contracts needed to address faster block times resulting from the [Cosmos SDK v0.50 upgrade](../axelard/2025-11-v1.3.0.md).

## Voting Verifier and Multisig migration

Migrate Multisig and all Voting Verifier contracts with updated poll/signing block expiry times. Note that these migrations should be executed _BEFORE_ the Axelard v1.3.0 upgrade.

- Create an `.env` config

```yaml
MNEMONIC=<cosm wasm deployer key mnemonic>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
```

    | Network          | `INIT_ADDRESSES`                                                                                                                                | `RUN_AS_ACCOUNT`                                | `DEPOSIT_VALUE` |
    | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------- |
    | devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                                 | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |
    | mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |

1. Store Voting Verifier code

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c VotingVerifier \
      -t "Store VotingVerifier contract v2.0.0" \
      -d "Store VotingVerifier contract v2.0.0" \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.0.0
    ```

1. Store XRPL Voting Verifier code

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c XrplVotingVerifier \
      -t "Store XrplVotingVerifier contract v2.0.0" \
      -d "Store XrplVotingVerifier contract v2.0.0" \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.0.0
    ```

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
    -r $RUN_AS_ACCOUNT \
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

| Network              | `epoch_duration` |
| -------------------- | ---------------- |
| **Devnet-amplifier** | `500`            |
| **Stagenet**         | `3000`           |
| **Testnet**          | `3000`           |
| **Mainnet**          | `74225`          |

1. Update epoch duration

    ```bash
    ts-node cosmwasm/migrate/update-rewards-pool-epoch-duration.ts update --epoch-duration [epoch-duration]
    ```

1. Verify epoch duration was updated on all pools
    ```bash
    ts-node cosmwasm/migrate/update-rewards-pool-epoch-duration.ts get-reward-pools
    ```
