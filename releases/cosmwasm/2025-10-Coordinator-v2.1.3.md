# Cosmwasm Coordinator v2.1.3

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>         |

| **Network**          | **Deployment Status** | **Date**   | **Coordinator** |
| -------------------- | --------------------- | ---------- | --------------- |
| **Devnet Amplifier** | -                     | -          | -               |
| **Stagenet**         | -                     | -          | -               |
| **Testnet**          | -                     | -          | -               |
| **Mainnet**          | -                     | -          | -               |


[Release](https://github.com/axelarnetwork/axelar-amplifier/tree/coordinator-v2.1.3)

## Background

Coordinator v2.1.3 adds support for Solana address format and encoder used in Solana Multisig Prover.

### Contract Version Info

| Contract             |  **Devnet**  | **Testnet** | **Stagenet** | **Mainnet** |
| -------------------- | ------------ | ----------- | ------------ | ----------- |
| `Coordinator`        | `2.1.2`      | `2.1.1`     | `2.1.1`      | `2.1.1`     |


## Deployment

1. Upload the coordinator contract

    | Network          | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                | `DEPOSIT_VALUE` |
    | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------- |
    | devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |
    | mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c Coordinator \
      -t "Upload Coordinator contract v2.1.3" \
      -d "Upload Coordinator contract v2.1.3" \
      -r $RUN_AS_ACCOUNT \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.1.3
    ```

1. Migrate to Coordinator v2.1.3

   There is no state migration needed during upgrade.

   ```bash
   ts-node cosmwasm/submit-proposal.js migrate \
   -c Coordinator \
   -t "Migrate Coordinator to v2.1.3" \
   -d "Coordinator to v2.1.3" \
   --msg '{}' \
   --fetchCodeId
   ```

## Checklist

1. Verify coordinator contract version

   ```bash
   ts-node cosmwasm/query.ts contract-info --contractName Coordinator -e $ENV
   ```
   Expected output

   ```bash
   {contract: 'coordinator', version: '2.1.3'}
   ```

1. Retrieve the service registry, router and multisig addresses from $ENV.json

   ```bash
   SERVICE_REGISTRY_ADDRESS=$(cat $ENV.json | jq -r .axelar.contracts.ServiceRegistry.address)
   ```

   ```bash
   ROUTER_ADDRESS=$(cat $ENV.json | jq -r .axelar.contracts.Router.address)
   ```

   ```bash
   MULTISIG_ADDRESS=$(cat $ENV.json | jq -r .axelar.contracts.Multisig.address)
   ```

1. Make sure the service registry, router and multisig addresses in the coordinator match the expected values.

   ```bash
   axelard q wasm contract-state raw --ascii $COORDINATOR_ADDRESS 'protocol' --node $NODE -o json | jq -r .data | base64 -d
   ```

   Expected output
   ```bash
   {"service_registry":"$SERVICE_REGISTRY_ADDRESS","router":"$ROUTER_ADDRESS","multisig":"$MULTISIG_ADDRESS"}
   ```

1. Check that the coordinator uses the same provers that the multisig uses for each chain.
   
   ```bash
   ts-node cosmwasm/migrate/migrate.ts check -e $ENV -c Coordinator 
   ```

   You may optionally specify the address of the coordinator and multisig by using the `--coordinator` and `--multisig` flags respectively.

   Expected Output
   ```bash
   ✅ Migration succeeded!
   ```
