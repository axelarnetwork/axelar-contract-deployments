# Cosmwasm Multisig v1.2.0

|                | **Owner**                                                               |
| -------------- | ----------------------------------------------------------------------- |
| **Created By** | @cjcobb23 <cj@interoplabs.io>                                           |
| **Deployment** | @isi8787 <isaac@interoplabs.io>, @blockchainguyy <ayush@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-05-06 |
| **Stagenet**         | Deployed              | 2025-05-08 |
| **Testnet**          | Deployed              | 2025-05-06 |
| **Mainnet**          | Deployed              | 2025-05-15 |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/interchain-token-service-v1.2.1)

## Background

Changes in this release:

1. Use error-stack for better error reporting

## Deployment

- This rollout upgrades the amplifier multisig contract from `v1.1.1` to `v1.2.0`
- There is a no state migration involved

1. Upload new ITS Hub contract

|   environment    |                                                               INIT_ADDRESSES                                                                |
| :--------------: | :-----------------------------------------------------------------------------------------------------------------------------------------: |
| devnet-amplifier |                        `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                        |
|     stagenet     | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
|     testnet      | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
|     mainnet      | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |

```bash
ts-node cosmwasm/submit-proposal.js store -c Multisig -t "Upload Multisig contract v1.2.0" -d "Upload Multisig contract v1.2.0" --instantiateAddresses $INIT_ADDRESSES --version 1.2.0
```

2. Upgrade Multisig contract

There is no state migration needed during upgrade.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c Multisig \
  -t "Migrate Multisig to v1.2.0" \
  -d "Multisig to v1.2.0" \
  --msg '{}' \
  --fetchCodeId
```

## Checklist

Verify multisig contract version

```bash
axelard query wasm contract-state raw $MULTISIG_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"multisig","version":"1.2.0"}
```
