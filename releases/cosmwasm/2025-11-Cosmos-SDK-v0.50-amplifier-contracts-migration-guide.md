# Cosmos SDK v0.50 Amplifier Contracts Migration Guide

|                | **Owner**                                                                 |
| -------------- | ------------------------------------------------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io>, @kulikthebird <tomasz@interoplabs.io> |
| **Deployment** |                                                                           |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

## Background

This document outlines parameter updates to Amplifier contracts needed to address faster block times resulting from the [Cosmos SDK v0.50 upgrade](../axelard/2025-11-v1.3.0.md).

## Voting Verifier and Multisig migration

Migrate Multisig and all Voting Verifier contracts with updated poll/signing block expiry times. Note that these migrations should be executed _BEFORE_ the Axelard v1.3.0 upgrade.

- Create an `.env` config

```yaml
MNEMONIC=<cosm wasm deployer key mnemonic>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
```

- TODO: get contract versions, build artifacts

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
      -r $RUN_AS_ACCOUNT \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.0.0
    ```

1. Store Multisig code

    ```bash
        ts-node cosmwasm/submit-proposal.js store \
        -c Multisig \
        -t "Upload Multisig contract v2.x.x" \ # get correct version
        -d "Upload Multisig contract v2.x.x" \ # get correct version
        -r $RUN_AS_ACCOUNT \
        --instantiateAddresses $INIT_ADDRESSES \
        --version 2.x.x # get correct version
    ```

1. Update `blockExpiry` on Multisig and all Voting Verifier contracts in chain config

    | Network          | new `blockExpiry` |
    | ---------------- | ----------------- |
    | devnet-amplifier | 50                |
    | stagenet         | TBD               |
    | testnet          | TBD               |
    | mainnet          | TBD               |

1. Migrate all VotingVerifier contracts

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts migrate-voting-verifiers \
    --fetchCodeId \
    -r $RUN_AS_ACCOUNT
    ```

1. Migrate Multisig contract

    ```bash
    ts-node cosmwasm/submit-proposal.js migrate \
    -c Multisig \
    -t "Migrate Multisig to v2.x.x" \ # get correct version
    -d "Multisig to v2.x.x" \ # get correct version
    --msg '{}' \
    -r $RUN_AS_ACCOUNT \
    --fetchCodeId
    ```

1. Verify Voting Verifier contract version
    - TODO: add script to check all VV migrations

1. Verify Multisig contract version

    ```bash
    ts-node cosmwasm/query.ts contract-info --contractName Multisig -e $ENV
    ```

    Expected output

    ```bash
    {"contract":"multisig","version":"2.x.x"} # get correct version
    ```

1. Update block expiry on all Voting Verifier contracts

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts update-voting-verifiers
    -r $RUN_AS_ACCOUNT
    ```

1. Update block expiry on Multisig contract

    ```bash
    ts-node cosmwasm/migrate/sdk50.ts update-signing-parameters-for-multisig
    -r $RUN_AS_ACCOUNT
    ```

1. TODO: verify updated params

## Reward pools epoch duration

Update `epoch_duration` on all reward pools. Note that these parameter changes should be executed _AFTER_ the Axelard v1.3.0 upgrade.

- TODO: add table for epoch duration by network after observing new block times
- TODO: add instructions for epoch duration update script
- TODO: add script to verify update
