# Cosmwasm Multisig v2.1.0

|  | **Owner** |
|-----------|------------|
| **Created By** | @cjcobb23 <cj@interoplabs.io> |
| **Deployment** | TBD |

| **Network** | **Deployment Status** | **Date** |
|-------------|----------------------|----------|
| **Devnet Amplifier** | Deployed | 05-10-2025 |
| **Stagenet** | - | TBD |
| **Testnet** | - | TBD |
| **Mainnet** | - | TBD |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-v2.1.0)

## Background

Changes in this release:

1. Accept arbitrary length message to sign. Allows for stateless sig verification callback.

## Deployment

- This rollout upgrades the amplifier multisig contract from `v2.0.0` to `v2.1.0`
- There is a no state migration involved

1. Upload new Multisig contract

| environment | INIT_ADDRESSES    |  RUN_AS_ACCOUNT |
| :-----: | :---: | :---: |
| devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`  | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |
| stagenet | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky`    | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |
| testnet | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7`   | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |
| mainnet | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am`   | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`   |

```bash
node cosmwasm/submit-proposal.js store -c Multisig -t "Upload Multisig contract v2.1.0" -d "Upload Multisig contract v2.1.0" -r $RUN_AS_ACCOUNT --deposit 2000000000 --instantiateAddresses $INIT_ADDRESSES --version 2.1.0
```

2. Upgrade Multisig contract

There is no state migration needed during upgrade.

```bash
node cosmwasm/submit-proposal.js migrate \
  -c Multisig \
  -t "Migrate Multisig to v2.1.0" \
  -d "Multisig to v2.1.0" \
  --msg '{}' \
  --fetchCodeId \
  --deposit 2000000000
```

## Checklist

Verify multisig contract version

```bash
axelard query wasm contract-state raw $MULTISIG_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```
Expected output

```bash
{"contract":"multisig","version":"2.1.0"}
```


