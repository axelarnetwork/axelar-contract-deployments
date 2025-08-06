# Cosmwasm Router v2.0.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @sdavidson1177 <solomon@interoplabs.io>         |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |



[Release](https://github.com/axelarnetwork/axelar-amplifier/tree/coordinator-v2.0.0)

## Background

Changes in this release:

1. The coordinator now stores both the router and multisig contract addresses in its state. This information will be given to the coordinator after it is instantiated using the *RegisterProtocol* message. The service registry address will also be registered using *RegisterProtocol*, where it was previously in the coordinator's instantiate message.
2. Previously, registering a chain with the coordinator involved specifying only the multisig prover's address. Now, registration must also include the corresponding gateway and voting verifier addresses.

## Deployment

- This rollout upgrades the amplifier coordinator contract from `v1.1.0` to `v2.0.0`
- State migration is required. The migration message must include the addresses of the router and the multisig contracts. The coordinator maintains a mapping between chain names and prover addresses. The corresponding gateway and voting verifier addresses for each of these provers must also be included in the migration message.

1. Upload new Coordinator contract

| Network          | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                | `DEPOSIT_VALUE` |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- | --------------- |
| devnet-amplifier | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/> `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
| stagenet         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm`<br/>`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/>`axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `100000000`     |
| testnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2`<br/>`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/>`axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |
| mainnet          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2`<br/>`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`<br/>`axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `2000000000`    |

```bash
ts-node cosmwasm/submit-proposal.js store -c Coordinator -t "Upload Coordinator contract v2.0.0" -d "Upload Coordinator contract v2.0.0" -r $RUN_AS_ACCOUNT --deposit $DEPOSIT_VALUE --instantiateAddresses $INIT_ADDRESSES --version 2.0.0
```

2. Get the **Multisig Prover**, **Gateway** and **Voting Verifier** for each Chain

First, you'll need to retrieve all chain/prover combinations tracked by the coordinator using the following command:

```bash
axelard q wasm contract-state all $COORDINATOR_ADDRESS -o json | jq -r '.models'
```

The output will be as follows:

```bash
[
  {
    "key": "0010636861696E5F70726F7665725F6D617062657261636861696E",
    "value": "ImF4ZWxhcjFrNDgzcTg5OHQ1dzBhY3F6eGhkamxzbW5wZ2N4eGE0OXllOG00Njc1N244bXRrNzB1Z3RzdTkyN3h3Ig=="
  },
  ...
]
```

Each key is hex encoded. When decoding each key as follows

```bash
echo "$KEY" | xxd -r -p
```

the output for some of the keys will be

```bash
chain_prover_map${CHAIN_NAME}
```

Each key with this format represents a $CHAIN_NAME to **multisig prover** mapping. The corresponding prover contract address is given as the value, and is base64 encoded. It can be retrieved in plaintext as follows:

```bash
echo "$VALUE" | base64 -d
```

Expected output

```bash
$MULTISIG_PROVER_ADDRESS
```

The **gateway** address for $CHAIN_NAME can be found by querying the router

```bash
./axelard q wasm contract-state smart $ROUTER_ADDRESS "'{\"chain_info\" : \"$CHAIN_NAME\"}'" -o json | jq -r '.data.gateway'
```

Expected output

```bash
{
  "address": "$GATEWAY_ADDRESS"
}
```

Finally, you can query the $MULTISIG_PROVER_ADDRESS to find the corresponding **voting verifier** address. Run the following command:

```bash
./axelard q wasm contract-state raw --ascii $MULTISIG_PROVER_ADDRESS 'config' -o json | jq -r '.data' | base64 -d | jq -r '.voting_verifier'
```

Expected output

```bash
$VOTING_VERIFIER_ADDRESS
```

3. Upgrade Coordinator contract

Provide chain names as well as multisig prover, gateway and voting verifier addresses to the coordinator.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c Coordinator \
  -t "Migrate Coordinator to v2.0.0" \
  -d "Migrate Coordinator to v2.0.0" \
  --msg "'{\"router\": \"$ROUTER_ADDRESS\", \"multisig\": \"$MULTISIG_ADDRESS\", \"chain_contracts\": [ \
    { \
        \"chain_name\": \"$CHAIN_NAME_1\", \
        \"prover_address\": \"$PROVER_ADDRESS_1\", \
        \"gateway_address\": \"$GATEWAY_ADDRESS_1\", \
        \"verifier_address\": \"$VERIFIER_ADDRESS_1\", \
    }, \
    { \
        \"chain_name\": \"$CHAIN_NAME_2\", \
        \"prover_address\": \"$PROVER_ADDRESS_2\", \
        \"gateway_address\": \"$GATEWAY_ADDRESS_2\", \
        \"verifier_address\": \"$VERIFIER_ADDRESS_2\", \
    }, \
    ...
  ]}'" \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
```

The migration endpoint enforces that a prover, gateway and verifier address is provided for all chains.

## Checklist

Verify coordinator contract version

```bash
axelard query wasm contract-state raw $COORDINATOR_ADDRESS 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```
Expected output

```bash
{"contract":"coordinator","version":"2.0.0"}
```

Verify multisig prover, gateway and voting verifier addresses stored on coordinator. For every chain name $CHAIN_NAME, do the following:

```bash
axelard q wasm contract-state smart $COORDINATOR_ADDRESS "'{\"chain_contracts_info\" : {\"chain_name\" : \"$CHAIN_NAME\"}}'" -o json | jq -r '.data'
```

Expected output

```bash
{
  "chain_name": "$CHAIN_NAME",
  "prover_address": "$PROVER_ADDRESS",
  "gateway_address": "$GATEWAY_ADDRESS",
  "verifier_address": "$VERIFIER_ADDRESS"
}
```

Ensure prover, gateway and verifier addresses match expected ones.

```bash
cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.MultisigProver[\"$CHAIN_NAME\"].address" | tr -d '"' | grep $PROVER_ADDRESS
cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Gateway[\"$CHAIN_NAME\"].address" | tr -d '"' | grep $GATEWAY_ADDRESS
cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.VotingVerifier[\"$CHAIN_NAME\"].address" | tr -d '"' | grep $VERIFIER_ADDRESS
```
