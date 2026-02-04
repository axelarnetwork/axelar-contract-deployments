# Solana GMP Amplifier

|                | **Owner**    |
| -------------- | ------------ |
| **Created By** | @nbayindirli |
| **Deployment** | @nbayindirli |

| **Axelar Env**       | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-12-04 |
| **Stagenet**         | Pending               | TBD        |
| **Testnet**          | Pending               | TBD        |
| **Mainnet**          | Pending               | TBD        |

- [Amplifier Fork](https://github.com/eigerco/axelar-amplifier)
- Contract Checksums:
    - SolanaMultisigProver: `9066612ae064702946322327eb6b3e7d450f45d0567ba7e35fb42ab4cded2349`

## Background

These are the instructions for deploying Amplifier contracts for Solana GMP connection.

### Pre-requisites

Ensure that the Solana gateway is deployed on Solana devnet/testnet/mainnet, as `VotingVerifier` needs the `sourceGatewayAddress` which is the External Gateway address.

## Build and Store Contracts

### Build Contracts

1. Clone and checkout the correct branch:

    ```bash
    git clone --recurse-submodules https://github.com/eigerco/axelar-amplifier.git axelar-amplifier-eiger
    cd axelar-amplifier-eiger
    git checkout main
    ```

1. Build the contracts and copy artifacts:

    ```bash
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/optimizer:0.17.0
    ```

1. Update the above Contract Checksums per `artifacts/checksums.txt`

That would create the `artifacts` folder with all the compiled contracts, plus the checksums.

### Store Contracts

Create an .env config:

```sh
MNEMONIC=xyz
ENV=xyz
CHAIN=<solana-custom|solana>
EIGER_ARTIFACT_PATH=../solana/axelar-amplifier-eiger/artifacts/
NODE=[Axelar RPC URL]
```

Add `INIT_ADDRESSES` to `.env`.

| Axelar Env           | `INIT_ADDRESSES`                                                                                                                                                                                              |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet-amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9,axelar1m2498n4h2tskcsmssjnzswl5e6eflmqnh487ds47yxyu6y5h4zuqr9zk4g`                                               |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky,axelar1nc3mfplae0atcchs9gqx9m6ezj5lfqqh2jmqx639kf8hd7m96lgq8a5e5y` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7,axelar1rwy79m8u76q2pm3lrxednlgtqjd8439l7hmctdxvjsv2shsu9meq8ntlvx` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am,axelar1rwy79m8u76q2pm3lrxednlgtqjd8439l7hmctdxvjsv2shsu9meq8ntlvx` |

```yaml
INIT_ADDRESSES=[INIT_ADDRESSES]
```

1. Store VotingVerifier:

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -c VotingVerifier \
        -t "Upload VotingVerifier contract for Solana" \
        -d "Upload VotingVerifier contract for Solana integration" \
        -v "2.0.0" \
        --chainName $CHAIN \
        -m $MNEMONIC \
        --instantiateAddresses $INIT_ADDRESSES \
        --governance
    ```

2. Store Gateway:

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -c Gateway \
        -t "Upload Gateway contract for Solana" \
        -d "Upload Gateway contract for Solana integration" \
        -v "1.1.1" \
        --chainName $CHAIN \
        -m $MNEMONIC \
        --instantiateAddresses $INIT_ADDRESSES \
        --governance
    ```

3. Store MultisigProver:

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -c MultisigProver \
        -t "Upload MultisigProver contract for Solana" \
        -d "Upload MultisigProver contract for Solana integration" \
        -a $EIGER_ARTIFACT_PATH \
        --chainName $CHAIN \
        -m $MNEMONIC \
        --instantiateAddresses $INIT_ADDRESSES \
        --governance
    ```

4. Store ItsSolanaTranslator:

    ```bash
    ts-node cosmwasm/contract.ts store-code \
        -c ItsSolanaTranslator \
        -t "Upload ItsSolanaTranslator contract v1.0.0" \
        -d "Upload ItsSolanaTranslator contract v1.0.0" \
        -a ../axelar-amplifier/artifacts \
        -m $MNEMONIC \
        --instantiateAddresses $INIT_ADDRESSES \
        --version 1.0.0 \
        --governance
    ```

5. Instantiate ItsSolanaTranslator:

    ```bash
    ts-node cosmwasm/contract.ts instantiate \
        -c ItsSolanaTranslator \
        -m $MNEMONIC \
        --fetchCodeId \
        --governance
    ```

6. After proposal passes, find the ItsSolanaTranslator contract address and update config if not auto-filled:

    ```bash
    # Get the code ID from the config
    CODE_ID=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.ItsSolanaTranslator.codeId")

    # List contracts instantiated from that code ID
    axelard q wasm list-contract-by-code $CODE_ID --node $NODE --output json | jq .

    # Update axelar.contracts.ItsSolanaTranslator.address in $ENV.json if not auto-filled
    ```

## Deployment

- Add config in `$ENV.json` to deploy Amplifier contracts.

| Axelar Env           | `governanceAddress`                             | `adminAddress`                                  |
| -------------------- | ----------------------------------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` |
| **Stagenet**         | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` |
| **Testnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` |
| **Mainnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |

| Axelar Env           | `serviceName` | `votingThreshold` | `signingThreshold` |
| -------------------- | ------------- | ----------------- | ------------------ |
| **Devnet-amplifier** | `validators`  | `["6", "10"]`     | `["6", "10"]`      |
| **Stagenet**         | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    |
| **Testnet**          | `amplifier`   | `["51", "100"]`   | `["51", "100"]`    |
| **Mainnet**          | `amplifier`   | `["2", "3"]`      | `["2", "3"]`       |

```bash
# Add under `config.axelar.contracts.VotingVerifier` based on Network
"$CHAIN" : {
  "governanceAddress": "[governance address]",
  "serviceName": "[service name]",
  "sourceGatewayAddress": "[external gateway PDA]",
  "votingThreshold": "[voting threshold]",
  "blockExpiry": 10,
  "confirmationHeight": 1000000,
  "msgIdFormat": "base58_solana_tx_signature_and_event_index",
  "addressFormat": "solana"
}
```

```bash
# Add under `config.axelar.contracts.SolanaMultisigProver` based on Network
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

### Instantiate Amplifier Contracts

| Axelar Env           | `CONTRACT_ADMIN`                                |
| -------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` |
| **Stagenet**         | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
| **Testnet**          | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| **Mainnet**          | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |

```bash
CONTRACT_ADMIN=[wasm contract admin address for the upgrade and migration based on network]
```

| Network              | Salt     |
| -------------------- | -------- |
| **Devnet-amplifier** | `v1.0.0` |
| **Stagenet**         | `v1.0.0` |
| **Testnet**          | `v1.0.0` |
| **Mainnet**          | `v1.0.0` |

1. Instantiate `Gateway`, `VotingVerifier` and `SolanaMultisigProver` contracts via Coordinator

    ```bash
    ts-node cosmwasm/contract.ts instantiate-chain-contracts \
        -n $CHAIN \
        -s $SALT \
        --fetchCodeId \
        --admin $CONTRACT_ADMIN \
        -m $MNEMONIC \
        --governance # omit on devnet-amplifier
    ```

1. Update the domainSeparator under `config.chains.$CHAIN.AxelarGateway`

1. Wait for proposal to pass and query deployed contract addresses

    ```bash
    ts-node cosmwasm/query.ts save-deployed-contracts $CHAIN
    ```

1. Register deployment

    ```bash
    ts-node cosmwasm/contract.ts register-deployment $CHAIN \
        -m $MNEMONIC \
        --governance # omit on devnet-amplifier
    ```

1. Set environment variables
    - Env-specific environment variables: These variables need to be updated by the env.

    ```bash
    VOTING_VERIFIER=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.VotingVerifier[\"$CHAIN\"].address" | tr -d '"')
    GATEWAY=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.Gateway[\"$CHAIN\"].address" | tr -d '"')
    MULTISIG_PROVER=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.SolanaMultisigProver[\"$CHAIN\"].address" | tr -d '"')
    MULTISIG=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.Multisig.address" | tr -d '"')
    REWARDS=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.Rewards.address" | tr -d '"')
    ROUTER=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.Router.address" | tr -d '"')
    SERVICE_REGISTRY=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.ServiceRegistry.address" | tr -d '"')
    ```

    - Gov proposal environment variables. Update these for each Axelar env

    | Axelar Env           | `PROVER_ADMIN`                                  | `REWARD_AMOUNT`     |
    | -------------------- | ----------------------------------------------- | ------------------- |
    | **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `1000000uamplifier` |
    | **Stagenet**         | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `1000000uaxl`       |
    | **Testnet**          | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `1000000uaxl`       |
    | **Mainnet**          | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` | `1000000uaxl`       |

    ```bash
    PROVER_ADMIN=[prover admin who is responsible for the contract's operations]
    REWARD_AMOUNT=[reward amount]
    ```

    - Add a community post for the mainnet proposal (i.e: <https://community.axelar.network/t/proposal-add-its-hub-to-mainnet/3227>) and share on `mainnet-announcements` channel on Discord.

    - Note: all the following governance proposals should be submitted at one time so deployment doesn't get held up while waiting for voting.

#### Verification

1. Verify Gateway Registration:

    ```bash
    axelard q wasm contract-state smart $ROUTER "{\"chain_info\": \"$CHAIN\"}" --output json --node $NODE | jq .
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
            "msg_id_format": "base58_solana_tx_signature_and_event_index"
        }
    }
    ```

1. Verify SolanaMultisigProver authorized on Multisig

    ```bash
    axelard q wasm contract-state smart $MULTISIG "{\"is_caller_authorized\": {\"contract_address\": \"$MULTISIG_PROVER\", \"chain_name\": \"$CHAIN\"}}" --output json --node $NODE | jq .

    # Result should look like:
    {
        "data": true
    }
    ```

#### Rewards

1. Create reward pools for VotingVerifier and Multisig

    | Axelar Env           | `epoch_duration` | `participation_threshold` | `rewards_per_epoch` |
    | -------------------- | ---------------- | ------------------------- | ------------------- |
    | **Devnet-amplifier** | `100`            | `[\"7\", \"10\"]`         | `100`               |
    | **Stagenet**         | `600`            | `[\"7\", \"10\"]`         | `100`               |
    | **Testnet**          | `14845`          | `[\"7\", \"10\"]`         | `100`               |
    | **Mainnet**          | `14845`          | `[\"8\", \"10\"]`         | TBD                 |

    ```bash
    ts-node cosmwasm/contract.ts create-reward-pools $CHAIN \
        --epochDuration "[epoch_duration]" \
        --participationThreshold "[participation_threshold]" \
        --rewardsPerEpoch "[rewards_per_epoch]" \
        --governance # omit on devnet-amplifier
    ```

1. Update ampd with the Solana chain configuration. Verifiers should use their own Solana RPC node for the `http_url` in production.

    | Axelar Env           | `rpc_url`                             |
    | -------------------- | ------------------------------------- |
    | **Devnet-amplifier** | `https://api.devnet.solana.com`       |
    | **Stagenet**         | `https://api.testnet.solana.com`      |
    | **Testnet**          | `https://api.testnet.solana.com`      |
    | **Mainnet**          | `https://api.mainnet-beta.solana.com` |

    ```bash
    [[handlers]]
      - type: MultisigSigner
        cosmwasm_contract: $MULTISIG
        chain_name: $CHAIN
      - type: SolanaMsgVerifier
        chain_name: $CHAIN
        cosmwasm_contract: $VOTING_VERIFIER
        rpc_url: [rpc_url]
        gateway_address: $GATEWAY
      - type: SolanaVerifierSetVerifier
        chain_name: $CHAIN
        cosmwasm_contract: $VOTING_VERIFIER
        rpc_url: [rpc_url]
        gateway_address: $GATEWAY
    ```

1. Update ampd with the Solana chain configuration.

    | Axelar Env           | `service_name` |
    | -------------------- | -------------- |
    | **Devnet-amplifier** | `validators`   |
    | **Stagenet**         | `amplifier`    |
    | **Testnet**          | `amplifier`    |
    | **Mainnet**          | `amplifier`    |

    ```bash
    ampd register-public-key ed25519

    ampd register-chain-support [service_name] $CHAIN
    ```

1. Add funds to reward pools from a wallet containing the reward funds `$REWARD_AMOUNT`

    ```bash
    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$MULTISIG\" } } }" --amount $REWARD_AMOUNT --from $WALLET --node $NODE
    axelard tx wasm execute $REWARDS "{ \"add_rewards\": { \"pool_id\": { \"chain_name\": \"$CHAIN\", \"contract\": \"$VOTING_VERIFIER\" } } }" --amount $REWARD_AMOUNT --from $WALLET --node $NODE

    # Check reward pool to confirm funding worked
    ts-node cosmwasm/query.ts rewards $CHAIN
    ```

1. Add public key to validator set

    ```bash
    axelard query wasm contract-state smart $SERVICE_REGISTRY "{ \"active_verifiers\": { \"service_name\": \"[service_name]\", \"chain_name\": \"$CHAIN\"} }" --node $NODE
    ```

1. Create genesis verifier set

    Note that this step can only be run once a sufficient number of verifiers have registered.

    ```bash
    axelard tx wasm execute $MULTISIG_PROVER '"update_verifier_set"' --from $PROVER_ADMIN --gas auto --gas-adjustment 1.2 --node $NODE

    # Query the multisig prover for active verifier set
    axelard q wasm contract-state smart $MULTISIG_PROVER '"current_verifier_set"' --node $NODE
    ```

- Return to 'Initialization Steps:Initialize Gateway' in the [Solana GMP](../solana/2025-09-GMP-v1.0.0.md)

## Checklist

- Follow the [Solana GMP checklist](../solana/2025-09-GMP-v1.0.0.md)
- Follow the [Solana ITS checklist](../solana/2025-09-ITS-v1.0.0.md)
