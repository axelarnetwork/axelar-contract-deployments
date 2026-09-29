# Robinhood CosmWasm GMP

|                | **Owner**                               |
| -------------- | --------------------------------------- |
| **Created By** | @makischristou <makis@commonprefix.com> |
| **Deployment** | @makischristou <makis@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**       |
| -------------------- | --------------------- | -------------- |
| **Devnet Amplifier** | -                     | TBD            |
| **Stagenet**         | -                     | TBD            |
| **Testnet**          | Complete              | September 2026 |
| **Mainnet**          | -                     | TBD            |

- [Amplifier Releases](https://github.com/axelarnetwork/axelar-amplifier/releases)
- [VotingVerifier v2.0.2](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/voting-verifier-v2.0.2)
- [Gateway v1.1.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/gateway-v1.1.1)
- [MultisigProver v1.2.1](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/multisig-prover-v1.2.1)

## Background

These are the instructions for deploying Amplifier contracts for the Robinhood connection.

### Pre-requisites

1. Predict the [External Gateway](../evm/2026-09-Robinhood-GMP.md) address, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

    | Network     | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
    | ----------- | ---------------------- | ---------------- | -------------------------------------------- |
    | **Testnet** | `3600`                 | `create`         | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

    ```bash
    ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay] --predictOnly
    ```

1. Coordinator contract must be deployed and configured in `$ENV.json`

## Deployment

- Create an `.env` config and set `CHAIN=robinhood` in your shell.

```yaml
MNEMONIC=<cosm wasm deployer key mnemonic>
ENV=testnet
CHAIN=robinhood
```

| Network     | `deployer address`                |
| ----------- | --------------------------------- |
| **Testnet** | `[governance-proposal-submitter]` |

- Confirm the following contract versions and code IDs are stored in `testnet.json`.

| Contract       | Version | Code ID |
| -------------- | ------- | ------- |
| VotingVerifier | `2.0.2` | `87`    |
| Gateway        | `1.1.1` | `24`    |
| MultisigProver | `1.2.1` | `85`    |

- Add config in `$ENV.json` to deploy Amplifier contracts.

| Network     | `governanceAddress`                             | `adminAddress`                                  |
| ----------- | ----------------------------------------------- | ----------------------------------------------- |
| **Testnet** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` |

| Network     | `serviceName` | `votingThreshold` | `signingThreshold` |
| ----------- | ------------- | ----------------- | ------------------ |
| **Testnet** | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    |

Add the following entries under `axelar.contracts`:

```json
{
    "VotingVerifier": {
        "robinhood": {
            "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
            "serviceName": "amplifier",
            "sourceGatewayAddress": "0x2a949565682ad89ca4Ad325499C36d3865a3ee02",
            "votingThreshold": ["51", "100"],
            "blockExpiry": 50,
            "confirmationHeight": 1,
            "msgIdFormat": "hex_tx_hash_and_event_index",
            "addressFormat": "eip55",
            "codeId": 87,
            "version": "2.0.2"
        }
    },
    "Gateway": {
        "robinhood": {
            "codeId": 24,
            "version": "1.1.1"
        }
    },
    "MultisigProver": {
        "robinhood": {
            "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
            "adminAddress": "axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf",
            "signingThreshold": ["51", "100"],
            "serviceName": "amplifier",
            "verifierSetDiffThreshold": 0,
            "encoder": "abi",
            "keyType": "ecdsa",
            "codeId": 85,
            "domainSeparator": "0x790b531d918bad40a2fac786b46518feaea4fec71b1d7f2a43cce0bb70e0a1fc",
            "version": "1.2.1"
        }
    }
}
```

### Instantiate Amplifier contracts

| Network     | `CONTRACT_ADMIN`                                | `SALT`                          |
| ----------- | ----------------------------------------------- | ------------------------------- |
| **Testnet** | `axelar1wxej3l9aczsns3harrtdzk7rct29jl47tvu8mp` | `Coordinator:robinhood:v1.0.13` |

`CONTRACT_ADMIN` is the wasm contract admin address for contract upgrades. Use the code IDs configured above; the Coordinator deployment name is `robinhood-24-87-85`.

1. Instantiate Gateway, VotingVerifier and MultisigProver contracts via Coordinator

    ```bash
    ts-node cosmwasm/contract.ts instantiate-chain-contracts \
    -n $CHAIN \
    -s "$SALT" \
    --admin $CONTRACT_ADMIN \
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

    | Network     | `PROVER_ADMIN`                                  | `REWARD_AMOUNT` |
    | ----------- | ----------------------------------------------- | --------------- |
    | **Testnet** | `axelar1w7y7v26rtnrj4vrx6q3qq4hfsmc68hhsxnadlf` | `1000000uaxl`   |

    ```bash
    PROVER_ADMIN=[prover admin who is responsible for the contract's operations]
    REWARD_AMOUNT=[reward amount]
    ```

    - For mainnet, add a community post for the proposal (i.e: <https://community.axelar.network/t/proposal-add-its-hub-to-mainnet/3227>) and share on `mainnet-announcements` channel on Discord.

    - Note: all the following governance proposals should be submitted at one time so deployment doesn't get held up while waiting for voting. [ITS proposal](../evm/2026-09-Robinhood-ITS-v2.2.0.md) should also be submitted at this time if possible.

1. Create reward pools for Voting Verifier and Multisig

    | Network     | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
    | ----------- | ---------------- | ------------------------- | ------------------- |
    | **Testnet** | `600`            | `[\"7\", \"10\"]`         | `100`               |

    ```bash
    ts-node cosmwasm/contract.ts create-reward-pools $CHAIN \
        --epochDuration "[epoch_duration]" \
        --participationThreshold "[participation_threshold]" \
        --rewardsPerEpoch "[rewards_per_epoch]" \
        --governance
    ```

1. Register ITS edge contract on ITS Hub

    Proceed with this step only if ITS deployment on Robinhood is confirmed. Add the following to `contracts` in the Robinhood config within `ENV.json`:

    | Network     | `ITS_EDGE_CONTRACT`                          |
    | ----------- | -------------------------------------------- |
    | **Testnet** | `0x3270540e5d2857E69BEC34c6EcFa02cD813aCb42` |

    ```json
    {
        "InterchainTokenService": {
            "address": "$ITS_EDGE_CONTRACT"
        }
    }
    ```

    ```bash
    ts-node cosmwasm/contract.ts its-hub-register-chains $CHAIN \
        --governance
    ```

    - Retain the deployed ITS address in the chain config after submitting the proposal.

1. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

    ```bash
    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET --keyring-backend test

    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET --keyring-backend test
    ```

1. Confirm proposals have passed
    - Check proposals on block explorer (i.e. <https://testnet.axelarscan.io/proposals>)
    - "Instantiate contracts for Robinhood"
    - "Create reward pools for Robinhood"
    - (optional) "Register Robinhood on ITS Hub"

    - Check Gateway registered at Router

    ```bash
    axelard q wasm contract-state smart $ROUTER "{\"chain_info\": \"$CHAIN\"}" --output json | jq .
    # You should see something like this:
    {
        "data": {
            "name": "robinhood",
            "gateway": {
                "address": "axelar1k4jsc5tf4tgdqupzj3a89a9glx63s9vuzrypfsc9qgmkmlymw8hss80c93"
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

1. Configure the modular EVM handler for Robinhood using the [ampd setup instructions](../ampd/2025-12-19-ampd-v1.14.2.md). Add the following chain entry to the ampd config:

    ```toml
    [[grpc.blockchain_service.chains]]
    chain_name = "robinhood"
    voting_verifier = "axelar1995v0ysau2cqqedjw2zh5luqlfpnn49hrhtdyhp9lxf72c765upsgupqld"
    multisig_prover = "axelar1r5yr5u8c6gdy75k3kffletp3clz5cg96e8h0sv8t5gew83f43z8q8dpvzz"
    multisig = "axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
    ```

    Set the handler's base config:

    ```toml
    ampd_url = "http://127.0.0.1:9090"
    chain_name = "robinhood"
    ```

    Set the EVM-specific handler config. Verifiers should use their own Robinhood RPC node in production.

    ```toml
    rpc_url = "https://rpc.testnet.chain.robinhood.com"
    finalization = "RPCFinalizedBlock"
    ```

1. Update `ampd` with the Robinhood chain configuration.

    ```bash
    ampd register-chain-support "[service name]" $CHAIN
    ```

1. Create genesis verifier set

    Note that this step can only be run once a sufficient number of verifiers have registered.

    | Network     | `min_num_verifiers` |
    | ----------- | ------------------- |
    | **Testnet** | 5                   |

    ```bash
    axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --keyring-backend test --gas auto --gas-adjustment 1.2
    ```

    Query the multisig prover for active verifier set

    ```bash
    axelard q wasm contract-state smart $MULTISIG_PROVER '"current_verifier_set"'
    ```

## Checklist

The [GMP checklist for Robinhood](../evm/2026-09-Robinhood-GMP.md) will test GMP calls.
