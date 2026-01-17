# Axelar Amplifier Role Transfers Upgrades

|                | **Owner**                                                               |
|----------------|-------------------------------------------------------------------------|
| **Created By** | @chipshort <christoph@interoplabs.io>                                           |
| **Deployment** |                                          |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |


[Router Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/router-v1.4.2)
[Multisig Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-v2.4.2)
[Interchain Token Service Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/interchain-token-service-v1.3.1)

## Background

This is a pre-requisite for the [role transfer](2025-11-Axelar-Amplifier-Role-Transfers-Release-v1.0.0.md#critical-contract-upgrade-required-before-role-transfers).
We need to be able to update the admin addresses of all the contracts and this upgrade adds the necessary message handlers.

Changes in this release:

- Adds `UpdateAdmin` message to Router, Multisig, and Interchain Token Service contracts.
- Adds `UpdateOperator` message to Interchain Token Service contract.

Upgrades Router to v1.4.2, Multisig to v2.4.2, and Interchain Token Service to v1.3.1. No migration message is required.

## Deployment

- This rollout upgrades the amplifier Router contract to `v1.4.2`, Multisig to `v2.4.2`, and Interchain Token Service to `v1.3.1`
- There is no state migration involved

1. Upload new Router, Multisig, and Interchain Token Service contracts

```bash
mkdir -p "/tmp/artifacts"
pushd /tmp/artifacts
wget https://static.axelar.network/releases/cosmwasm/router/1.4.2/router.wasm
wget https://static.axelar.network/releases/cosmwasm/multisig/2.4.2/multisig.wasm
wget https://static.axelar.network/releases/cosmwasm/interchain-token-service/1.3.1/interchain_token_service.wasm
popd

ts-node cosmwasm/submit-proposal.js store --artifact-dir /tmp/artifacts -t "Upload Router v1.4.2, Multisig v2.4.2, and Interchain Token Service v1.3.1" -d "Upload Router v1.4.2, Multisig v2.4.2, and Interchain Token Service v1.3.1" -c Router Multisig InterchainTokenService

rm -r /tmp/artifacts
```

2. Migrate contracts

```bash
ts-node cosmwasm/submit-proposal.js migrate -c Router -t "Migrate Router to v1.4.2" -d "Migrate Router to v1.4.2" --msg '{}' --fetchCodeId
ts-node cosmwasm/submit-proposal.js migrate -c Multisig -t "Migrate Multisig to v2.4.2" -d "Migrate Multisig to v2.4.2" --msg '{}' --fetchCodeId
ts-node cosmwasm/submit-proposal.js migrate -c InterchainTokenService -t "Migrate Interchain Token Service to v1.3.1" -d "Migrate Interchain Token Service to v1.3.1" --msg '{}' --fetchCodeId
```

## Checklist

- [ ] Verify router contract version

  ```bash
  axelard query wasm contract-state raw $ROUTER_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
  ```
  Expected output

  ```bash
  {"contract":"router","version":"1.4.2"}
  ```
- [ ] Verify multisig contract version
  ```bash
  axelard query wasm contract-state raw $MULTISIG_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
  ```
  Expected output

  ```bash
  {"contract":"multisig","version":"2.4.2"}
  ```
- [ ] Verify interchain token service contract version

  ```bash
  axelard query wasm contract-state raw $INTERCHAIN_TOKEN_SERVICE_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
  ```
  Expected output

  ```bash
  {"contract":"interchain-token-service","version":"1.3.1"}
  ```
