# Cosmwasm Multisig v2.3.2

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io> |
| **Deployment** | @isi8787 <isaac@interoplabs.io>                 |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Complete              | 2025-11-17 |
| **Stagenet**         | Complete              | 2025-11-17|
| **Testnet**          | -                     | -          |
| **Mainnet**          | -                     | -          |


[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-v2.3.2)

## Background

Changes in this release:

This is a patch on top of Multisig 2.3.x. There are no functional changes to the contract. This release was created
to migrate the Multisig contracts on each network to the same version (from 2.3.x to 2.3.2).

## Deployment

- This rollout upgrades the amplifier multisig contract from `v2.3.x` to `v2.3.2`
- There is no state migration involved

1. Upload new Multisig contract

    | Network          | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                | `DEPOSIT_VALUE` |
    | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------- |
    | devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |
    | mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c Multisig \
      -t "Upload Multisig contract v2.3.2" \
      -d "Upload Multisig contract v2.3.2" \
      -r $RUN_AS_ACCOUNT \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.3.2
    ```

2. Upgrade Multisig contract

There is no state migration needed during upgrade.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c Multisig \
  -t "Migrate Multisig to v2.3.2" \
  -d "Multisig to v2.3.2" \
  --msg '{}' \
  --fetchCodeId
```

## Checklist

Verify multisig contract version

```bash
ts-node cosmwasm/query.ts contract-info --contractName Multisig -e $ENV
```

Expected output

```bash
{"contract":"multisig","version":"2.3.2"}
```
