# Stellar GMP Amplifier

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @ahramy <ahram@interoplabs.io>       |
| **Deployment** | @RiceAndMeet <steven@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-03-20 |
| **Stagenet**         | Completed             | 2025-03-20 |
| **Testnet**          | Completed             | 2025-03-20 |
| **Mainnet**          | Completed             | 2025-03-21 |

- [Amplifier Releases](https://github.com/axelarnetwork/axelar-amplifier/releases)
- [VotingVerifier v1.1.0](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v1.1.0)
- [Gateway v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)
- [MultisigProver v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.1.1)

## Background

These are the instructions for deploying Amplifier contracts for Stellar connection.

### Pre-requisites

Ensure that the [External Gateway](../stellar/2025-01-GMP-v1.0.0.md) is deployed first, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

## Deployment

- Create an `.env` config. `CHAIN` should be set to `stellar` for mainnet, and `stellar-2024-q4` for all other networks.

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

`CONTRACT_ADMIN` is the wasm contract admin address for contract upgrades.

| Network              | Salt     |
| -------------------- | -------- |
| **Devnet-amplifier** | `v1.0.0` |
| **Stagenet**         | `v1.0.0` |
| **Testnet**          | `v1.0.0` |
| **Mainnet**          | `v1.0.0` |

**Note:** On `devnet-amplifier`, omit the `--governance` flag to execute directly (it uses a governance key for direct execution).

1. Instantiate Gateway, VotingVerifier and MultisigProver contracts via Coordinator

    ```bash
    ts-node cosmwasm/contract.ts instantiate-chain-contracts \
    -n $CHAIN \
    -s "$SALT" \
    --admin $CONTRACT_ADMIN \
    --fetchCodeId \
    --governance # omit on devnet-amplifier
    ```

1. Wait for proposal to pass and query deployed contract addresses

    ```bash
    ts-node cosmwasm/query.ts save-deployed-contracts $CHAIN
    ```

1. Register deployment

    ```bash
    ts-node cosmwasm/contract.ts register-deployment $CHAIN \
    --governance # omit on devnet-amplifier
    ```

### Submit proposals

1. Set environment variables
    - These variables are network-specific

    ```bash
    VOTING_VERIFIER=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.VotingVerifier[\"$CHAIN\"].address" | tr -d '"')
    GATEWAY=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.Gateway[\"$CHAIN\"].address" | tr -d '"')
    MULTISIG_PROVER=$(cat ./axelar-chains-config/info/$ENV.json | jq ".axelar.contracts.MultisigProver[\"$CHAIN\"].address" | tr -d '"')
    MULTISIG=$(cat ./axelar-chains-config/info/$ENV.json | jq .axelar.contracts.Multisig.address | tr -d '"')
    REWARDS=$(cat ./axelar-chains-config/info/$ENV.json | jq .axelar.contracts.Rewards.address | tr -d '"')
    ROUTER=$(cat ./axelar-chains-config/info/$ENV.json | jq .axelar.contracts.Router.address | tr -d '"')
    ```

    - Gov proposal environment variables. Update these for each network

    | Network              | `PROVER_ADMIN`                                  | `REWARD_AMOUNT`     |
    | -------------------- | ----------------------------------------------- | ------------------- |
    | **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `1000000uamplifier` |
    | **Stagenet**         | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `1000000uaxl`       |
    | **Testnet**          | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `1000000uaxl`       |
    | **Mainnet**          | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` | `1000000uaxl`       |

    ```bash
    PROVER_ADMIN=[prover admin who is responsible for the contract's operations]
    REWARD_AMOUNT=[reward amount]
    ```

    - Add a community post for the mainnet proposal. i.e: <https://community.axelar.network/t/proposal-add-its-hub-to-mainnet/3227>

    - Note: all the following governance proposals should be submitted at one time so deployment doesn't get held up while waiting for voting.

#### Rewards

1. Create reward pools for VotingVerifier and Multisig

    | Network              | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
    | -------------------- | ---------------- | ------------------------- | ------------------- |
    | **Devnet-amplifier** | `100`            | `[\"7\", \"10\"]`         | `100`               |
    | **Stagenet**         | `600`            | `[\"7\", \"10\"]`         | `100`               |
    | **Testnet**          | `14845`          | `[\"7\", \"10\"]`         | `100`               |
    | **Mainnet**          | `14845`          | `[\"8\", \"10\"]`         | `920000000`         |

    ```bash
    ts-node cosmwasm/contract.ts create-reward-pools $CHAIN \
        --epochDuration "[epoch_duration]" \
        --participationThreshold "[participation_threshold]" \
        --rewardsPerEpoch "[rewards_per_epoch]" \
        --governance # omit on devnet-amplifier
    ```

1. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

    ```bash
    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET

    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET
    ```

1. Confirm proposals have passed
    - Check proposals on block explorer (i.e. <https://axelarscan.io/proposals>)
        - "Instantiate contracts for stellar"
        - "Register deployment for stellar"
        - "Create reward pools for stellar"

    - Check Gateway registered at Router

    ```bash
    axelard q wasm contract-state smart $ROUTER "{\"chain_info\": \"$CHAIN\"}" --output json | jq .
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

    - Check Multisig Prover authorized on Multisig

    ```bash
    axelard q wasm contract-state smart $MULTISIG "{\"is_caller_authorized\": {\"contract_address\": \"$MULTISIG_PROVER\", \"chain_name\": \"$CHAIN\"}}" --output json | jq .
    # Result should look like:
    {
        "data": true
    }
    ```

    - Check reward pool to confirm funding worked:

    ```bash
    ts-node cosmwasm/query.ts rewards $CHAIN
    ```

1. Update `ampd` with the Stellar chain configuration. Verifiers should use their own Stellar RPC node for the `http_url` in production.

    | Network              | `http_url`                             |
    | -------------------- | -------------------------------------- |
    | **Devnet-amplifier** | `https://horizon-testnet.stellar.org/` |
    | **Stagenet**         | `https://horizon-testnet.stellar.org/` |
    | **Testnet**          | `https://horizon-testnet.stellar.org/` |
    | **Mainnet**          | `https://horizon.stellar.org`          |

    ```bash
    [[handlers]]
    type="StellarMsgVerifier"
    http_url=[http_url]
    cosmwasm_contract="$VOTING_VERIFIER"

    [[handlers]]
    type="StellarVerifierSetVerifier"
    http_url=[http_url]
    cosmwasm_contract="$VOTING_VERIFIER"
    ```

1. Update `ampd` with the Stellar chain configuration.

    ```bash
    ampd register-public-key ed25519

    ampd register-chain-support "[service name]" $CHAIN
    ```

1. Create genesis verifier set

    Note that this step can only be run once a sufficient number of verifiers have registered.

    | Network              | `min_num_verifiers` |
    | -------------------- | ------------------- |
    | **Devnet-amplifier** | 3                   |
    | **Stagenet**         | 3                   |
    | **Testnet**          | 5                   |
    | **Mainnet**          | 5                   |

    ```bash
    axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2
    ```

    Query the multisig prover for active verifier set

    ```bash
    axelard q wasm contract-state smart $MULTISIG_PROVER '"current_verifier_set"'
    ```

## Checklist

The [Stellar GMP checklist](../stellar/2025-01-GMP-v1.0.0.md) will test GMP call.
