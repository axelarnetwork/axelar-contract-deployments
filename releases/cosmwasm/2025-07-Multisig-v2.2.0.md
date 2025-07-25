# Cosmwasm Router v1.3.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>         |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| ...| ...             | ... |



<!-- [Release]() -->

## Background

Changes in this release:

1. Multisig stores coordinator's address. This address is given when the multisig contract is instantiated. This allows the multisig to give the coordinator permission to execute messages (such as when authorizing callers).

## Deployment

- This rollout upgrades the amplifier multisig contract from `v2.1.0` to `v2.2.0`
- State migration is required. The multisig must be supplied with the coordinator's address

1. Upload new Multisig contract

```bash
ts-node cosmwasm/submit-proposal.js store -c Multisig -t "Upload Multisig contract v2.2.0" -d "Upload Multisig contract v2.2.0" --version 2.2.0
```

2. Upgrade Multisig contract

Provide coordinator address to the multisig.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c Multisig \
  -t "Migrate Multisig to v2.2.0" \
  -d "Multisig to v2.2.0" \
  --msg '{\"coordinator\": \"$COORDINATOR_ADDRESS\"}' \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
```

## Checklist

Verify multisig contract version

```bash
axelard query wasm contract-state raw $MULTISIG_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```
Expected output

```bash
{"contract":"multisig","version":"2.2.0"}
```

Verify coordinator address stored on multisig

```bash
axelard q wasm contract-state raw --ascii $MULTISIG_ADDRESS 'config' -o json | jq -r '.data' | base64 -d | jq -r '.coordinator'
```

Expected output

```bash
$COORDINATOR_ADDRESS
```
