# Cosmwasm Coordinator v2.1.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>         |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |


[Release](https://github.com/axelarnetwork/axelar-amplifier/tree/coordinator-v2.1.0)

## Background

The coordinator can now deploy a gateway, voting verifier, and multisig prover contract for a given chain. It can then register these contracts with the router and multisig in a separate transaction, thereby completing that chain’s integration with GMP. These new functionalities introduced in coordinator v2.1.0 require the router to be upgraded to version v1.3.0, and the multisig to be upgraded to v2.3.0. Listed below are the relevant changes made to each contract.

### Contract Version Info

| Contract             |  **Devnet**  | **Testnet** | **Stagenet** | **Mainnet** |
| -------------------- | ------------ | ----------- | ------------ | ----------- |
| `Coordinator`        | `1.1.0`      | `1.1.0`     | `1.1.0`      | `1.1.0`     |
| `Multisig`           | `2.1.0`      | `2.1.0`     | `2.1.0`      | `2.1.0`     |
| `Router`             | `1.2.0`      | `1.2.0`     | `1.2.0`      | `1.2.0`     |


### Coordinator v2.1.0

1. The coordinator now stores both the router and multisig contract addresses in its state. This information will be given to the coordinator after it is instantiated using the *RegisterProtocol* message. The service registry address will also be registered using *RegisterProtocol*, where it was previously in the coordinator's instantiate message.
1. Previously, registering a chain with the coordinator involved specifying only the multisig prover's address. Now, registration must also include the corresponding gateway and voting verifier addresses.

### Multisig v2.3.0

1. Multisig stores the coordinator address. This address is given when the multisig contract is instantiated. This allows the multisig to give the coordinator permission to execute messages (such as when authorizing callers).
1. Added the `AuthorizedCaller` endpoint. This allows the authorized caller (prover contract) for any given chain to be queried.

### Router v1.3.0

1. Router contract stores the coordinator address. This address is given when the router contract is instantiated. This allows the router to give the coordinator permission to execute message (such as when registering chains).

## Deployment

- This rollout upgrades the amplifier coordinator contract from `v1.1.0` to `v2.1.0`, the multisig contract from `v2.1.0` to `v2.3.0`, and the router from `v1.2.0` to `v1.3.0`.
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
      -t "Upload Multisig contract v2.3.0" \
      -d "Upload Multisig contract v2.3.0" \
      -r $RUN_AS_ACCOUNT \
      --deposit $DEPOSIT_VALUE \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.3.0
    ```

    ```bash
    ts-node cosmwasm/submit-proposal.js store \
      -c Coordinator \
      -t "Upload Coordinator contract v2.1.0" \
      -d "Upload Coordinator contract v2.1.0" \
      -r $RUN_AS_ACCOUNT \
      --deposit $DEPOSIT_VALUE \
      --instantiateAddresses $INIT_ADDRESSES \
      --version 2.1.0
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
   ts-node cosmwasm/submit-proposal.js migrate \
     -c Multisig \
     -t "Migrate Multisig to v2.3.0" \
     -d "Multisig to v2.3.0" \
     --msg "{\"coordinator\": \"$COORDINATOR_ADDRESS\"}" \
     --fetchCodeId \
     --deposit $DEPOSIT_VALUE
   ```

1. Migrate to Coordinator v2.1.0 using the contract deployment scripts

   ```bash
   ts-node cosmwasm/migrate/migrate.ts <coordinator_code_id> \
      --address $COORDINATOR_ADDRESS \
      -m $MNEMONIC \
      -d $DEPOSIT_VALUE
   ```

   This script generates the migration message, and submits the migration proposal. You may use the `--dry` flag to only generate the migration message.

## Checklist

1. Verify router contract version

   ```bash
   ts-node cosmwasm/contract.ts info --contract Router -e $ENV
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
   ts-node cosmwasm/contract.ts info --contract Multisig -e $ENV
   ```
   Expected output

   ```bash
   {contract: 'multisig', version: '2.3.0'}
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
   ts-node cosmwasm/contract.ts info --contract Coordinator -e $ENV
   ```
   Expected output

   ```bash
   {contract: 'coordinator', version: '2.1.0'}
   ```
