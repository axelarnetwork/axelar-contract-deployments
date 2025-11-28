# Cosmwasm Service Registry v1.2.1

|                | **Owner**                                                               |
|----------------|-------------------------------------------------------------------------|
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>                                           |
| **Deployment** | @sdavidson1177 <solomon@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
|----------------------|-----------------------|------------|
| **Devnet Amplifier** | --                    | --         |
| **Stagenet**         | --                    | --         |
| **Testnet**          | --                    | --         |
| **Mainnet**          | --                    | --         |


[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/service-registry-v1.2.1)

## Background

Changes in this release:

1. Stores the authorized verifier count for all existing services.

Migrating to v1.2.0 does not create an entry for existing services in the AUTHORIZED_VERIFIER_COUNT structure. This causes the service registry to throw an error when trying to authorize or unauthorize verifiers for existing services. This migration fixes this problem by storing the number of authorized verifiers for all existing services.

## Deployment

This rollout upgrades the amplifier service registry contract from `v1.2.0` to `v1.2.1`.

1. Upload new Service Registry contract

```bash
ts-node cosmwasm/submit-proposal.js store -c ServiceRegistry -t "Upload Service Registry contract v1.2.1" -d "Upload Service Registry contract v1.2.1" --version 1.2.1
```

2. Upgrade Service Registry contract

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c ServiceRegistry \
  -t "Migrate Service Registry to v1.2.1" \
  -d "Service Registry to v1.2.1" \
  --msg '{}' \
  --fetchCodeId
```

## Checklist

Verify the authorized verifier count for the following services:

| **Network**          | **Service**           | **Query**   |
|----------------------|-----------------------|------------|
| **Devnet Amplifier** | validators            | 0019617574686F72697A65645F76657269666965725F636F756E7476616C696461746F7273 |
| **Stagenet**         | amplifier             | 0019617574686f72697a65645f76657269666965725f636f756e74616d706c6966696572   |
| **Testnet**          | amplifier             | 0019617574686f72697a65645f76657269666965725f636f756e74616d706c6966696572   |
| **Mainnet**          | amplifier             | 0019617574686f72697a65645f76657269666965725f636f756e74616d706c6966696572   |

For reference, the query is constructed as follows:

```bash
0019 + hex(authorized_verifier_count) + hex($SERVICE_NAME)
```

Perform the query as follows:

```bash
axelard query wasm contract-state raw $SERVICE_REGISTRY_ADDRESS $QUERY -o json | jq -r '.data'
```
The expected output should not be null. You can decode this output using

```bash
axelard query wasm contract-state raw $SERVICE_REGISTRY_ADDRESS $QUERY -o json | jq -r '.data' | base64 -d
```

to get the authorized verifier count.
