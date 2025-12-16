# Celo-Sepolia GMP Amplifier v6.0.6

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io> |
| **Deployment** | @AttissNgo <attiss@interoplabs.io> |

| **Network**  | **Deployment Status** | **Date**   |
| ------------ | --------------------- | ---------- |
| **Stagenet** | Completed             | 2025-09-05 |
| **Testnet**  | Completed             | 2025-09-05 |

- [Amplifier Releases](https://github.com/axelarnetwork/axelar-amplifier/releases)
- [VotingVerifier v1.1.0](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v1.1.0)
- [Gateway v1.1.0](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)
- [MultisigProver v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.1.1)

## Background

Celo Sepolia will replace Alfajores testnet when Holesky sunsets in September 2025. These are the instructions for deploying Amplifier contracts for `Celo-Sepolia` connection in stagenet and testnet. Mainnet is not affected.

### Pre-requisites

Predict the [External Gateway](../evm/2025-09-Celo-Sepolia-GMP-v6.0.6.md) address, as `VotingVerifier` deployment requires the `sourceGatewayAddress` which is the External Gateway address.

| Network      | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
| ------------ | ---------------------- | ---------------- | -------------------------------------------- |
| **Stagenet** | `300`                  | `create`         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**  | `3600`                 | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |

```bash
ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay] --predictOnly
```

## Deployment

- Create an `.env` config

```yaml
MNEMONIC=<cosm wasm deployer key mnemonic>
ENV=<stagenet|testnet>
CHAIN=celo-sepolia
```

| Network      | `deployer address`                              |
| ------------ | ----------------------------------------------- |
| **Stagenet** | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm` |
| **Testnet**  | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2` |

- Confirm `VotingVerifier`, `Gateway` and `MultisigProver` contracts are already stored in `$ENV.json`

```bash
VotingVerifier (v1.1.0) -> "storeCodeProposalCodeHash": "d9412440820a51bc48bf41a77ae39cfb33101ddc6562323845627ea2042bf708"
Gateway (v1.1.1) -> "storeCodeProposalCodeHash": "2ba600ee0d162184c9387eaf6fad655f1d75db548f93e379f0565cb2042d856f"
MultisigProver (v1.1.1) -> "storeCodeProposalCodeHash": "00428ef0483f103a6e1a5853c4b29466a83e5b180cc53a00d1ff9d022bc2f03a"
```

- Add config in `$ENV.json` to deploy Amplifier contracts.

| Network      | `governanceAddress`                             | `adminAddress`                                  |
| ------------ | ----------------------------------------------- | ----------------------------------------------- |
| **Stagenet** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` |
| **Testnet**  | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |

| Network      | `serviceName` | `votingThreshold` | `signingThreshold` | `confirmationHeight` |
| ------------ | ------------- | ----------------- | ------------------ | -------------------- |
| **Stagenet** | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    | `1`                  |
| **Testnet**  | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    | `1`                  |

```bash
# Add under `config.axelar.contracts.VotingVerifier` based on Network
"$CHAIN" : {
  "governanceAddress": "[governance address]",
  "serviceName": "amplifier",
  "sourceGatewayAddress": "[external gateway address]",
  "votingThreshold": ["51", "100"],
  "blockExpiry": 10,
  "confirmationHeight": 1000000,
  "msgIdFormat": "hex_tx_hash_and_event_index",
  "addressFormat": "eip55"
}

# Add under `config.axelar.contracts.MultisigProver` based on Network
"$CHAIN" : {
  "governanceAddress": "[governance address]",
  "adminAddress": "[admin address]",
  "signingThreshold": ["51", "100"],
  "serviceName": "amplifier",
  "verifierSetDiffThreshold": 0,
  "encoder": "abi",
  "keyType": "ecdsa"
}
```

### Instantiate Amplifier contracts

| Network      | `CONTRACT_ADMIN`                                |
| ------------ | ----------------------------------------------- |
| **Stagenet** | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
| **Testnet**  | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |

`CONTRACT_ADMIN` is the wasm contract admin address for contract upgrades.

| Network      | Salt     |
| ------------ | -------- |
| **Stagenet** | `v1.0.0` |
| **Testnet**  | `v1.0.0` |

1. Instantiate Gateway, VotingVerifier and MultisigProver contracts via Coordinator

    ```bash
    ts-node cosmwasm/contract.ts instantiate-chain-contracts \
    -n $CHAIN \
    -s "$SALT" \
    --admin $CONTRACT_ADMIN \
    --fetchCodeId \
    --governance
    ```

1. Wait for proposal to pass and query deployed contract addresses

    ```bash
    ts-node cosmwasm/query.ts save-deployed-contracts $CHAIN
    ```

1. Register deployment

    ```bash
    ts-node cosmwasm/contract.ts register-deployment $CHAIN \
    --governance
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

    | Network      | `PROVER_ADMIN`                                  | `REWARD_AMOUNT` |
    | ------------ | ----------------------------------------------- | --------------- |
    | **Stagenet** | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `1000000uaxl`   |
    | **Testnet**  | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `1000000uaxl`   |

    ```bash
    PROVER_ADMIN=[prover admin who is responsible for the contract's operations]
    REWARD_AMOUNT=[reward amount]
    ```

    - Note: all the following governance proposals should be submitted at one time so deployment doesn't get held up while waiting for voting. [ITS proposal](../evm/EVM-ITS-Release-Template.md) should also be submitted at this time if possible.

#### Rewards

1. Create reward pools for VotingVerifier and Multisig

    | Network      | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
    | ------------ | ---------------- | ------------------------- | ------------------- |
    | **Stagenet** | `600`            | `[\"7\", \"10\"]`         | `100`               |
    | **Testnet**  | `600`            | `[\"7\", \"10\"]`         | `100`               |

    ```bash
    ts-node cosmwasm/contract.ts create-reward-pools $CHAIN \
        --epochDuration "[epoch_duration]" \
        --participationThreshold "[participation threshold]" \
        --rewardsPerEpoch "[rewards per epoch]" \
        --governance
    ```

1. Register ITS edge contract on ITS Hub

    Proceed with this step only if ITS deployment on $CHAIN is confirmed. Add the following to `contracts` in the `$CHAIN`config within`ENV.json`:

    | Network      | `ITS_EDGE_CONTRACT`                          |
    | ------------ | -------------------------------------------- |
    | **Stagenet** | `0x0FCb262571be50815627C16Eca1f5F3D342FF5a5` |
    | **Testnet**  | `0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C` |

    ```json
    {
        "InterchainTokenService": {
            "address": "$ITS_EDGE_CONTRACT"
        }
    }
    ```

    ```bash
    ts-node cosmwasm/contract.ts its-hub-register-chains \
        -n $CHAIN \
        --governance
    ```

    - Please remove this temporary config after submitting the proposal and reset contracts to an empty object.

1. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

    ```bash
    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET

    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET
    ```

1. Confirm proposals have passed
    - Check proposals on block explorer (i.e. <https://axelarscan.io/proposals>)
        - "Instantiate contracts for $CHAIN"
        - "Register deployment for $CHAIN"
        - "Create reward pools for $CHAIN"
        - (optional) "Register $CHAIN on ITS Hub"

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

1. Update `ampd` with the `$CHAIN` chain configuration. Verifiers should use their own `$CHAIN` RPC node for the `http_url` in production.

    | Network      | `http_url`        |
    | ------------ | ----------------- |
    | **Stagenet** | [testnet RPC URL] |
    | **Testnet**  | [testnet RPC URL] |

    ```bash
    [[handlers]]
    chain_name="$CHAIN"
    cosmwasm_contract="$MULTISIG"
    type="MultisigSigner"

    [[handlers]]
    chain_finalization="RPCFinalizedBlock"
    chain_name="$CHAIN"
    chain_rpc_url=[http url]
    cosmwasm_contract="$VOTING_VERIFIER"
    type="EvmMsgVerifier"

    [[handlers]]
    chain_finalization="RPCFinalizedBlock"
    chain_name="$CHAIN"
    chain_rpc_url=[http url]
    cosmwasm_contract="$VOTING_VERIFIER"
    type="EvmVerifierSetVerifier"
    ```

1. Update `ampd` with the `$CHAIN` chain configuration.

    ```bash
    ampd register-chain-support "[service name]" $CHAIN
    ```

1. Create genesis verifier set

    Note that this step can only be run once a sufficient number of verifiers have registered.

    | Network      | `min_num_verifiers` |
    | ------------ | ------------------- |
    | **Stagenet** | 3                   |
    | **Testnet**  | 5                   |

    ```bash
    axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2
    ```

    Query the multisig prover for active verifier set

    ```bash
    axelard q wasm contract-state smart $MULTISIG_PROVER '"current_verifier_set"'
    ```

## Checklist

The [GMP checklist for $CHAIN](../evm/2025-09-Celo-Sepolia-GMP-v6.0.6.md) will test GMP calls.
