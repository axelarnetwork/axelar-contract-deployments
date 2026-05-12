# Cosmwasm Multisig v2.0.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @cjcobb23 <cj@interoplabs.io>          |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-05-10 |
| **Stagenet**         | Deployed              | 2025-05-20 |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | Deployed              | 2025-05-20 |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-v1.2.1)

## Background

Changes in this release:

1. Change custom signature verification to use an execute msg instead of a query.

## Deployment

- This rollout upgrades the amplifier multisig contract from `v1.2.1` to `v2.0.0`
- There is a no state migration involved

1. Upload new Multisig contract

```bash
ts-node cosmwasm/contract.ts store-code -c Multisig -t "Upload Multisig contract v2.0.0" -d "Upload Multisig contract v2.0.0" --version 2.0.0 --governance
```

2. Upgrade Multisig contract

There is no state migration needed during upgrade.

```bash
ts-node cosmwasm/contract.ts migrate \
  -c Multisig \
  -t "Migrate Multisig to v2.0.0" \
  -d "Multisig to v2.0.0" \
  --msg '{}' \
  --fetchCodeId \
  --governance
```

## Checklist

Verify multisig contract version

```bash
axelard query wasm contract-state raw $MULTISIG_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"multisig","version":"2.0.0"}
```
