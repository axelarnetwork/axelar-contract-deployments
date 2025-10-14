# Cosmwasm Coordinator v2.1.1

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>         |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-10-01 |
| **Stagenet**         | Completed             | 2025-10-14 |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |


[Release](https://github.com/axelarnetwork/axelar-amplifier/tree/coordinator-v2.1.1)

## Background

The coordinator can now deploy a gateway, voting verifier, and multisig prover contract for a given chain. It can then register these contracts with the router and multisig in a separate transaction, thereby completing that chain’s integration with GMP. These new functionalities introduced in coordinator v2.1.1 require the router to be upgraded to version v1.3.0, and the multisig to be upgraded to v2.3.1. Listed below are the relevant changes made to each contract.

### Contract Version Info

| Contract             |  **Devnet**  | **Testnet** | **Stagenet** | **Mainnet** |
| -------------------- | ------------ | ----------- | ------------ | ----------- |
| `Coordinator`        | `1.1.0`      | `1.1.0`     | `1.1.0`      | `1.1.0`     |
| `Multisig`           | `2.1.0`      | `2.1.0`     | `2.1.0`      | `2.1.0`     |
| `Router`             | `1.2.0`      | `1.2.0`     | `1.2.0`      | `1.2.0`     |


### Coordinator v2.1.1

1. The coordinator now stores both the router and multisig contract addresses in its state. This information will be given to the coordinator after it is instantiated using the *RegisterProtocol* message. The service registry address will also be registered using *RegisterProtocol*, where it was previously in the coordinator's instantiate message.
1. Previously, registering a chain with the coordinator involved specifying only the multisig prover's address. Now, registration must also include the corresponding gateway and voting verifier addresses.

### Multisig v2.3.1

1. Multisig stores the coordinator address. This address is given when the multisig contract is instantiated. This allows the multisig to give the coordinator permission to execute messages (such as when authorizing callers).
1. Added the `AuthorizedCaller` endpoint. This allows the authorized caller (prover contract) for any given chain to be queried.
1. Multisig can no longer have multiple provers registered for a particular chain.

### Router v1.3.0

1. Router contract stores the coordinator address. This address is given when the router contract is instantiated. This allows the router to give the coordinator permission to execute message (such as when registering chains).

## Deployment

- This rollout upgrades the amplifier coordinator contract from `v1.1.0` to `v2.1.1`, the multisig contract from `v2.1.0` to `v2.3.1`, and the router from `v1.2.0` to `v1.3.0`.
- State migration is required for all three contracts.

1. Retrieve coordinator address from the appropriate config file for the environment. (ENV: devnet, testnet, stagenet or mainnet)

   ```bash
   export COORDINATOR_ADDRESS=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Coordinator.address" | tr -d '"')
   ```

1. Upload the new router, multisig and coordinator contracts

    | Network          | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                | `DEPOSIT_VALUE` |
    | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------- |
    | devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
    | testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |
    | mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c Router \
      -t "Upload Router contract v1.3.0" \
      -d "Upload Router contract v1.3.0" \
      -r $RUN_AS_ACCOUNT \
      --deposit $DEPOSIT_VALUE \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 1.3.0
    ```

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c Multisig \
      -t "Upload Multisig contract v2.3.1" \
      -d "Upload Multisig contract v2.3.1" \
      -r $RUN_AS_ACCOUNT \
      --deposit $DEPOSIT_VALUE \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.3.1
    ```

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c Coordinator \
      -t "Upload Coordinator contract v2.1.1" \
      -d "Upload Coordinator contract v2.1.1" \
      -r $RUN_AS_ACCOUNT \
      --deposit $DEPOSIT_VALUE \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.1.1
    ```

1. Upgrade the router and multisig before upgrading the coordinator

   Provide coordinator address to the router.

   ```bash
   ts-node cosmwasm/submit-proposal.js migrate \
     -c Router \
     -t "Migrate Router to v1.3.0" \
     -d "Router to v1.3.0" \
     --msg "{\"coordinator\": \"$COORDINATOR_ADDRESS\"}" \
     --fetchCodeId \
     --deposit $DEPOSIT_VALUE
   ```

   Provide coordinator address to the multisig.

   ```bash
   ts-node cosmwasm/migrate/migrate.ts migrate \
      --address $MULTISIG_ADDRESS \
      -m $MNEMONIC \
      --deposit $DEPOSIT_VALUE
   ```

   The `default_authorized_provers` object should correspond to the chain/prover pairs located at `axelar.contracts.MultisigProver` in `$ENV.json`.

1. Migrate to Coordinator v2.1.1 using the contract deployment scripts

   ```bash
   ts-node cosmwasm/migrate/migrate.ts migrate \
      --address $COORDINATOR_ADDRESS \
      -m $MNEMONIC \
      --deposit $DEPOSIT_VALUE
   ```

   This script generates the migration message, and submits the migration proposal. You may use the `--dry` flag to only generate the migration message.

   **Warning:** Using the `--ignoreChains [chains to ignore...]` flag might introduce protocol breaking behaviour, so it should be used only in a test environment. Coordinator v2 requires the gateways, verifiers and provers for each chain to be unique. You may ignore chains in the event that there are multiple chains that use the same verifier. This is possible because the protocol allows different gateways to be instantiated with the same verifier.

## Checklist

1. Verify router contract version

   ```bash
   ts-node cosmwasm/query.ts contract-info --contractName Router -e $ENV
   ```
   Expected output

   ```bash
   {contract: 'router', version: '1.3.0'}
   ```

1. Verify the coordinator address is stored on the router

   ```bash
   axelard q wasm contract-state raw --ascii $ROUTER_ADDRESS 'config' -o json | jq -r '.data' | base64 -d | jq -r '.coordinator'
   ```

   Expected output

   ```bash
   $COORDINATOR_ADDRESS
   ```

1. Ensure the coordinator address matches the predicted one.

   ```bash
   cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Coordinator.address" | tr -d '"' | grep $COORDINATOR_ADDRESS
   ```

1. Verify multisig contract version

   ```bash
   ts-node cosmwasm/query.ts contract-info --contractName Multisig -e $ENV
   ```
   Expected output

   ```bash
   {contract: 'multisig', version: '2.3.1'}
   ```

1. Verify the coordinator address is stored on the multisig

   ```bash
   axelard q wasm contract-state raw --ascii $MULTISIG_ADDRESS 'config' -o json | jq -r '.data' | base64 -d | jq -r '.coordinator'
   ```

   Expected output

   ```bash
   $COORDINATOR_ADDRESS
   ```

1. Ensure the coordinator address matches the predicted one.

   ```bash
   cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Coordinator.address" | tr -d '"' | grep $COORDINATOR_ADDRESS
   ```

1. Verify coordinator contract version

   ```bash
   ts-node cosmwasm/query.ts contract-info --contractName Coordinator -e $ENV
   ```
   Expected output

   ```bash
   {contract: 'coordinator', version: '2.1.1'}
   ```

1. Check that the coordinator uses the same provers that the multisig uses for each chain.
   
   ```bash
   ts-node cosmwasm/migrate/migrate.ts check -e $ENV -c Coordinator 
   ```

   You may manually specify the address of the coordinator and multisig by using the `--coordinator` and `--multisig` flags respectively.
