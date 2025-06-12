# Solana GMP Amplifier

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @eigerco                             |
| **Deployment** | @attiss                              |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | -          |
| **Stagenet**         | Completed             | 2025-06-11 |
| **Testnet**          | -                     | -          |
| **Mainnet**          | -                     | -          |

- [Amplifier Fork](https://github.com/eigerco/axelar-amplifier/tree/solana-cosmwasm)
- Contract Checksums:
  - VotingVerifier: `23063bd7064298e07e78fa208b521ea3d42bfa4036127e38c02ef1434ee84f91`
  - Gateway: `769b6695060da742c2ba578cdfe728c6e4438b90bf0df02477defc0b2641798c`
  - MultisigProver: `3a114cd8960d5fe529e1b0a4b47da0786419dbd1cbf4ffd33c7de7832725ba53`

## Background

These are the instructions for deploying Amplifier contracts for Solana GMP connection.

### Pre-requisites

Ensure that the Solana gateway is deployed on Solana devnet/testnet/mainnet, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

## Build and Store Contracts

### Build Contracts

1. Clone and checkout the correct branch:
```bash
git clone --recurse-submodules https://github.com/eigerco/axelar-amplifier.git
cd axelar-amplifier
git checkout solana-cosmwasm
```

2. Build the contracts and copy artifacts:
```bash
cd contracts/voting-verifier
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --lib
cd ../gateway
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --lib
cd ../multisig-prover
RUSTFLAGS='-C link-arg=-s' cargo build --release --target wasm32-unknown-unknown --lib

cd ../..
mkdir -p artifacts
cp target/wasm32-unknown-unknown/release/voting_verifier.wasm artifacts/
cp target/wasm32-unknown-unknown/release/gateway.wasm artifacts/
cp target/wasm32-unknown-unknown/release/multisig_prover.wasm artifacts/
```

### Store Contracts

Create an .env config:
```yaml
MNEMONIC=xyz
ENV=xyz
CHAIN=solana
ARTIFACT_PATH=../solana/axelar-amplifier/artifacts/
```

| Network              | `DEPOSIT_VALUE` |
| -------------------- | --------------- |
| **Devnet-amplifier** | `100000000`     |
| **Stagenet**         | `100000000`     |
| **Testnet**          | `2000000000`    |
| **Mainnet**          | `2000000000`    |

Add `INIT_ADDRESSES` to `.env`.

| Network              | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |

> **_NOTE:_**
> Set `RUN_AS_ACCOUNT` to an EOA account's address instead of the governance address to avoid having to instantiate the contracts via another governance proposal.

```yaml
INIT_ADDRESSES=
RUN_AS_ACCOUNT=
```

1. Store Voting Verifier:
```bash
ts-node cosmwasm/submit-proposal.js store \
  -c VotingVerifier \
  -t "Upload VotingVerifier contract for Solana" \
  -d "Upload VotingVerifier contract for Solana integration" \
  -a "$ARTIFACT_PATH/voting_verifier.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

2. Store Gateway:
```bash
ts-node cosmwasm/submit-proposal.js store \
  -c Gateway \
  -t "Upload Gateway contract for Solana" \
  -d "Upload Gateway contract for Solana integration" \
  -a "$ARTIFACT_PATH/gateway.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

3. Store Multisig Prover:\
```bash
ts-node cosmwasm/submit-proposal.js store \
  -c MultisigProver \
  -t "Upload MultisigProver contract for Solana" \
  -d "Upload MultisigProver contract for Solana integration" \
  -a "$ARTIFACT_PATH/multisig_prover.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

## Deployment

- Add config in `$ENV.json` to deploy Amplifier contracts.

| Network              | `governanceAddress`                             | `adminAddress`                                  |
| -------------------- | ----------------------------------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` |
| **Stagenet**         | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` |
| **Testnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| **Mainnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |

| Network              | `serviceName` | `votingThreshold` | `signingThreshold` | `confirmationHeight` |
| -------------------- | ------------- | ----------------- | ------------------ | -------------------- |
| **Devnet-amplifier** | `validators`  | `["6", "10"]`     | `["6", "10"]`      | `1`                  |
| **Stagenet**         | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    | `1`                  |
| **Testnet**          | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    | `1`                  |
| **Mainnet**          | `amplifier`   | `["2", "3"]`      | `["2", "3"]`       | `1`                  |

```bash
# Add under `config.axelar.contracts.VotingVerifier` based on Network
\"$CHAIN\" : {
  "governanceAddress": "[governance address]",
  "serviceName": "[service name]",
  "sourceGatewayAddress": "[external gateway address]",
  "votingThreshold": "[voting threshold]",
  "blockExpiry": 10,
  "confirmationHeight": 31,
  "msgIdFormat": "base58_solana_tx_signature_and_event_index",
  "addressFormat": "base58_solana"
}

# Add under `config.axelar.contracts.MultisigProver` based on Network
\"$CHAIN\" : {
  "governanceAddress": "[governance address]",
  "adminAddress": "[admin address]",
  "signingThreshold": "[signing threshold]",
  "serviceName": "[service name]",
  "verifierSetDiffThreshold": 0,
  "encoder": "solana",
  "keyType": "ecdsa"
}
```

### Instantiate Amplifier contracts

| Network              | `CONTRACT_ADMIN`                                |
| -------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` |
| **Stagenet**         | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
| **Testnet**          | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| **Mainnet**          | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |

```bash
CONTRACT_ADMIN=[wasm contract admin address for the upgrade and migration based on network]
```

1. Instantiate `VotingVerifier`

```bash
ts-node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

2. Instantiate `Gateway`

```bash
ts-node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

3. Instantiate `MultisigProver`

```bash
ts-node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

4. Set environment variables

- Network-specific environment variables: These variables need to be updated by the network.

```bash
VOTING_VERIFIER=$(cat "./axelar-chains-config/info/\"$ENV\".json" | jq ".axelar.contracts.VotingVerifier[\"$CHAIN\"].address" | tr -d '"')
GATEWAY=$(cat "./axelar-chains-config/info/\"$ENV\".json" | jq ".axelar.contracts.Gateway[\"$CHAIN\"].address" | tr -d '"')
MULTISIG_PROVER=$(cat "./axelar-chains-config/info/\"$ENV\".json" | jq ".axelar.contracts.MultisigProver[\"$CHAIN\"].address" | tr -d '"')
MULTISIG=$(cat "./axelar-chains-config/info/\"$ENV\".json" | jq ".axelar.contracts.Multisig.address" | tr -d '"')
REWARDS=$(cat "./axelar-chains-config/info/\"$ENV\".json" | jq ".axelar.contracts.Rewards.address" | tr -d '"')
ROUTER=$(cat "./axelar-chains-config/info/\"$ENV\".json" | jq ".axelar.contracts.Router.address" | tr -d '"')
```

- Gov proposal environment variables. Update these for each network

| Network              | `PROVER_ADMIN`                                  | `DEPOSIT_VALUE` | `REWARD_AMOUNT`     |
| -------------------- | ----------------------------------------------- | --------------- | ------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `100000000`     | `1000000uamplifier` |
| **Stagenet**         | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `100000000`     | `1000000uaxl`       |
| **Testnet**          | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `2000000000`    | `1000000uaxl`       |
| **Mainnet**          | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` | `2000000000`    | `1000000uaxl`       |

```bash
PROVER_ADMIN=[prover admin who is responsible for the contract's operations]
DEPOSIT_VALUE=[deposit value]
REWARD_AMOUNT=[reward amount]
RUN_AS_ACCOUNT=[wasm deployer/governance address]
```

- `--runAs $RUN_AS_ACCOUNT` is only required for devnet-amplifier. Do not use `--runAs` for stagenet, testnet, or mainnet.
- Add a community post for the mainnet proposal. i.e: https://community.axelar.network/t/proposal-add-its-hub-to-mainnet/3227

5. Register Gateway at the Router

```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for solana" \
  -d "Register Gateway address for solana at Router contract" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"register_chain\": {
      \"chain\": \"$CHAIN\",
      \"gateway_address\": \"$GATEWAY\",
      \"msg_id_format\": \"hex_tx_hash_and_event_index\"
      }
    }"
```

- Verify Gateway Registration:

```bash
axelard q wasm contract-state smart $ROUTER "{\"chain_info\": \"$CHAIN\"}" --output json --node [axelar rpc address] | jq .
```

```bash
# You should see something like this:
{
  "data": {
    "name": \"$CHAIN\",
    "gateway": {
      "address": "axelar1jah3ac59xke2r266yjhh45tugzsvnlzsefyvx6jgp0msk6tp7vqqaktuz2"
    },
    "frozen_status": 0,
    "msg_id_format": "hex_tx_hash_and_event_index"
  }
}
```

6. Register prover contract on coordinator

```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for solana" \
  -d "Register Multisig Prover address for solana at Coordinator contract" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"register_prover_contract\": {
      \"chain_name\": \"$CHAIN\",
      \"new_prover_addr\": \"$MULTISIG_PROVER\"
    }
  }"
```

7. Authorize Multisig prover on Multisig

```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for solana" \
  -d "Authorize Multisig Prover address for solana at Multisig contract" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"authorize_callers\": {
      \"contracts\": {
        \"$MULTISIG_PROVER\": \"$CHAIN\"
      }
    }
  }"
```

```bash
axelard q wasm contract-state smart $MULTISIG "{\"is_caller_authorized\": {\"contract_address\": \"$MULTISIG_PROVER\", \"chain_name\": \"$CHAIN\"}}" --output json | jq .

# Result should look like:
{
  "data": true
}
```

8. Create reward pool for voting verifier

#### Rewards

| Network              | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
| -------------------- | ---------------- | ------------------------- | ------------------- |
| **Devnet-amplifier** | `100`            | `[\"7\", \"10\"]`         | `100`               |
| **Stagenet**         | `600`            | `[\"7\", \"10\"]`         | `100`               |
| **Testnet**          | `14845`          | `[\"7\", \"10\"]`         | `100`               |
| **Mainnet**          | `14845`          | `[\"8\", \"10\"]`         | `920000000`         |

```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for solana in solana voting verifier" \
  -d "Create pool for solana in solana voting verifier" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"<epoch_duration>\",
        \"participation_threshold\": [<participation_threshold>],
        \"rewards_per_epoch\": \"<rewards_per_epoch>\"
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$VOTING_VERIFIER\"
      }
    }
  }"
```

9. Create reward pool for multisig

```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for solana in axelar multisig" \
  -d "Create pool for solana in axelar multisig" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"create_pool\": {
      \"params\": {
        \"epoch_duration\": \"<epoch_duration>\",
        \"participation_threshold\": [<participation_threshold>],
        \"rewards_per_epoch\": \"<rewards_per_epoch>\"
      },
      \"pool_id\": {
        \"chain_name\": \"$CHAIN\",
        \"contract\": \"$MULTISIG\"
      }
    }
  }"
```

10. Update ampd with the Solana chain configuration. Verifiers should use their own Solana RPC node for the `http_url` in production.

| Network              | `http_url`                             |
| -------------------- | -------------------------------------- |
| **Devnet-amplifier** | `solana devnet rpc` |
| **Stagenet**         | `solana devnet rpc` |
| **Testnet**          | `solana devnet rpc` |
| **Mainnet**          | `solana mainnet rpc`          |

```bash
[[handlers]]
type="SolanaMsgVerifier"
http_url=[http_url]
cosmwasm_contract="$VOTING_VERIFIER"

[[handlers]]
type="SolanaVerifierSetVerifier"
http_url=[http_url]
cosmwasm_contract="$VOTING_VERIFIER"
```

11. Update ampd with the Solana chain configuration.

```bash
TODO: ampd register-public-key ???

ampd register-chain-support "[service name]" $CHAIN
```

12. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

```bash
axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET
axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET

# Check reward pool to confirm funding worked
ts-node cosmwasm/query.js rewards -n $CHAIN
```

13. Create genesis verifier set

Note that this step can only be run once a sufficient number of verifiers have registered.

```bash
axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2

# Query the multisig prover for active verifier set
axelard q wasm contract-state smart $MULTISIG_PROVER '"current_verifier_set"'
```

## Checklist

* Follow the [Solana GMP checklist](../solana/2025-05-GMP-v1.0.0.md)
* Follow the [Solana ITS checklist](../solana/2025-05-ITS-v1.0.0.md)