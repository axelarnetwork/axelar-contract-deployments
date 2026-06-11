# Cosmwasm Service Registry v1.2.2

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @rista404 <ristic@commonprefix.com>  |
| **Deployment** | @rista404 <ristic@commonprefix.com>  |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2026-06-11 |
| **Stagenet**         | Completed             | 2026-06-11 |
| **Testnet**          | Completed             | 2026-06-11 |
| **Mainnet**          | --                    | --         |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/service-registry-v1.2.2)

## Background

Changes in this release:

1. Reject `bond_verifier` calls with multi-denom funds ([#1165](https://github.com/axelarnetwork/axelar-amplifier/pull/1165))
2. Require verifier to be authorized before registering for chain support ([#1166](https://github.com/axelarnetwork/axelar-amplifier/pull/1166))

## Deployment

This rollout upgrades the amplifier service registry contract from `v1.2.x` to `v1.2.2`. The migration message is empty; the migrate entry point backfills the authorized verifier count for any services missing it (idempotent).

1. Upload new Service Registry contract

```bash
ts-node cosmwasm/contract.ts store-code -c ServiceRegistry -t "Upload Service Registry contract v1.2.2" -d "Upload Service Registry contract v1.2.2" --version 1.2.2 --governance
```

2. Upgrade Service Registry contract

```bash
ts-node cosmwasm/contract.ts migrate \
  -c ServiceRegistry \
  --msg '{}' \
  --fetchCodeId \
  --governance
```

## Checklist

Verify service registry contract version

```bash
axelard query wasm contract-state raw $SERVICE_REGISTRY_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"service-registry","version":"1.2.2"}
```
