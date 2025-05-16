# Solana GMP Amplifier

|                | **Owner**                      |
| -------------- | ------------------------------ |
| **Created By** | @<GITHUB_USERNAME> (<EMAIL>)   |
| **Deployment** | @<GITHUB_USERNAME> (<EMAIL>)   |

| **Network**          | **Deployment Status** | **Date**         |
| -------------------- | --------------------- | ---------------- |
| **Devnet Amplifier** | Pending               | <DEPLOY_DATE>    |
| **Stagenet**         | Pending               | <DEPLOY_DATE>    |
| **Testnet**          | Pending               | <DEPLOY_DATE>    |
| **Mainnet**          | Pending               | <DEPLOY_DATE>    |

- [Amplifier Releases](https://github.com/axelarnetwork/axelar-amplifier/releases)
- [VotingVerifier v1.1.0](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v1.1.0)
- [Gateway v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)
- [MultisigProver v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.1.1)

## Background

These are the instructions for deploying Amplifier contracts for Solana connection.

### Pre-requisites

Ensure that the [External Gateway](../solana/2025-05-GMP-v1.0.0.md) is deployed first, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

## Deployment

- Create an `.env` config. `CHAIN` should be set to `Solana`.

```yaml
MNEMONIC=xyz
ENV=xyz
CHAIN=xyz
```

- Confirm `VotingVerifier(v1.1.0)`, `Gateway(v1.1.1)` and `MultisigProver(v1.1.1)` contracts are already stored in `$ENV.json`

```bash
VotingVerifier(v1.1.0) -> "storeCodeProposalCodeHash": "d9412440820a51bc48bf41a77ae39cfb33101ddc6562323845627ea2042bf708"
Gateway(v1.1.1) -> "storeCodeProposalCodeHash": "2ba600ee0d162184c9387eaf6fad655f1d75db548f93e379f0565cb2042d856f"
MultisigProver(v1.1.1) -> "storeCodeProposalCodeHash": "00428ef0483f103a6e1a5853c4b29466a83e5b180cc53a00d1ff9d022bc2f03a"
```

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
  "confirmationHeight": 1,
  "msgIdFormat": "hex_tx_hash_and_event_index",
  "addressFormat": "stellar"
}

# Add under `config.axelar.contracts.MultisigProver` based on Network
\"$CHAIN\" : {
  "governanceAddress": "[governance address]",
  "adminAddress": "[admin address]",
  "signingThreshold": "[signing threshold]",
  "serviceName": "[service name]",
  "verifierSetDiffThreshold": 0,
  "encoder": "stellar_xdr",
  "keyType": "ed25519"
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
node ./cosmwasm/deploy-contract.js instantiate -c VotingVerifier --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

2. Instantiate `Gateway`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c Gateway --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
```

3. Instantiate `MultisigProver`

```bash
node ./cosmwasm/deploy-contract.js instantiate -c MultisigProver --fetchCodeId --instantiate2 --admin $CONTRACT_ADMIN
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

5. Register stellar gateway at the Router

```bash
node cosmwasm/submit-proposal.js execute \
  -c Router \
  -t "Register Gateway for Solana" \
  -d "Register Gateway address for Solana at Router contract" \
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

- Approve Proposal (must be done within 5 minutes on devnet-amplifier/stagenet and 1 hour on testnet/mainnet)

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
node cosmwasm/submit-proposal.js execute \
  -c Coordinator \
  -t "Register Multisig Prover for Solana" \
  -d "Register Multisig Prover address for Solana at Coordinator contract" \
  --runAs $RUN_AS_ACCOUNT \
  --deposit $DEPOSIT_VALUE \
  --msg "{
    \"register_prover_contract\": {
      \"chain_name\": \"$CHAIN\",
      \"new_prover_addr\": \"$MULTISIG_PROVER\"
    }
  }"
```

7. Authorize Solana Multisig prover on Multisig

```bash
node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Authorize Multisig Prover for Solana" \
  -d "Authorize Multisig Prover address for Solana at Multisig contract" \
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
node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for Solana in Solana voting verifier" \
  -d "Create pool for Solana in Solana voting verifier" \
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
node cosmwasm/submit-proposal.js execute \
  -c Rewards \
  -t "Create pool for Solana in axelar multisig" \
  -d "Create pool for Solana in axelar multisig" \
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
| **Devnet-amplifier** | `https://api.testnet.solana.com`       |
| **Stagenet**         | `https://api.testnet.solana.com`       |
| **Testnet**          | `https://api.testnet.solana.com`       |
| **Mainnet**          | `https://api.mainnet-beta.solana.com`  |

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
ampd register-public-key ed25519

ampd register-chain-support "[service name]" $CHAIN
```

12. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

```bash
axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET
axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET

# Check reward pool to confirm funding worked
node cosmwasm/query.js rewards -n $CHAIN
```

13. Create genesis verifier set

Note that this step can only be run once a sufficient number of verifiers have registered.

```bash
axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2

# Query the multisig prover for active verifier set
axelard q wasm contract-state smart $MULTISIG_PROVER '"current_verifier_set"'
```

## Checklist

The [Solana GMP checklist](../solana/2025-05-GMP-v1.0.0.md) will test GMP call.
