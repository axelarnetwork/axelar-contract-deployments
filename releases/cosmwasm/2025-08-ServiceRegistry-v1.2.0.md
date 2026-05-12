# Cosmwasm Service Registry v1.2.0

|                | **Owner**                                                               |
| -------------- | ----------------------------------------------------------------------- |
| **Created By** | @cjcobb23 <cj@interoplabs.io>                                           |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @isi8787 <isaac@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-08-28 |
| **Stagenet**         | Deployed              | 2025-08-28 |
| **Testnet**          | Deployed              | 2025-08-29 |
| **Mainnet**          | Deployed              | 2025-09-05 |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/service-registry-v1.2.0)

## Background

Changes in this release:

1. Adds support for chain specific minimum and maximum verifier set sizes. If set, these chain specific values override the service default values.

## Deployment

- This rollout upgrades the amplifier service registry contract to `v1.2.0`
- There is no state migration involved

1. Upload new Service Registry contract

```bash
ts-node cosmwasm/contract.ts store-code -c ServiceRegistry -t "Upload Service Registry contract v1.2.0" -d "Upload Service Registry contract v1.2.0" --version 1.2.0 --governance
```

2. Upgrade Service Registry contract

There is no state migration needed during upgrade.

```bash
ts-node cosmwasm/contract.ts migrate \
  -c ServiceRegistry \
  -t "Migrate Service Registry to v1.2.0" \
  -d "Service Registry to v1.2.0" \
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
{"contract":"service-registry","version":"1.2.0"}
```
