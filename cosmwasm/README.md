### Cosmwasm deployment scripts

This folder contains deployment scripts for cosmwasm contracts needed for amplifier.

### Setup

`npm ci && npm run build`

1. Compile the contracts in the amplifier [repo](https://github.com/axelarnetwork/axelar-amplifier) using the [rust optimizer](https://github.com/CosmWasm/rust-optimizer) for cosmwasm.

2. Add a `contracts` object to the `axelar` section of your config. Change any values as necessary. For chain specific contracts (`VotingVerifier`,`Gateway`,`MultisigProver`), there should be one object per chain, where the key is the chain id.

```
  "axelar": {
    "contracts": {
      "Coordinator": {
        "governanceAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz"
      },
      "ServiceRegistry": {
        "governanceAccount": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz"
      },
      "Router": {
        "adminAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
        "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"
      },
      "Multisig": {
        "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
        "blockExpiry": 10
      },
      "Rewards": {
        "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
        "rewardsDenom": "uamplifier",
        "params": {
          "epoch_duration": "10",
          "rewards_per_epoch": "100",
          "participation_threshold": [
            "9",
            "10"
          ]
        }
      },
      "NexusGateway": {
        "nexus": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz"
      },
      "VotingVerifier": {
        "ethereum-2": {
          "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
          "serviceName": "validators",
          "sourceGatewayAddress": "0xe432150cce91c13a887f7D836923d5597adD8E31",
          "votingThreshold": [
            "9",
            "10"
          ],
          "blockExpiry": 10,
          "confirmationHeight": 1
        },
        "Avalanche": {
          "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
          "serviceName": "validators",
          "sourceGatewayAddress": "0xe432150cce91c13a887f7D836923d5597adD8E31",
          "votingThreshold": [
            "9",
            "10"
          ],
          "blockExpiry": 10,
          "confirmationHeight": 1
        }
      },
      "Gateway": {
        "ethereum-2": {
        },
        "Avalanche": {
        }
      },
      "MultisigProver": {
        "ethereum-2": {
          "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
          "adminAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
          "destinationChainID": "0",
          "signingThreshold": [
            "4",
            "5"
          ],
          "serviceName": "validators",
          "workerSetDiffThreshold": 1,
          "encoder": "abi",
          "keyType": "ecdsa"
        },
        "Avalanche": {
          "adminAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
          "destinationChainID": "0",
          "signingThreshold": [
            "4",
            "5"
          ],
          "serviceName": "validators",
          "workerSetDiffThreshold": 1,
          "encoder": "abi",
          "keyType": "ecdsa"
        }
      }
    },

    "rpc": [rpc],
    "tokenSymbol": "amplifier",
    "gasPrice": "0.00005uamplifier",
    "gasLimit": 5000000
  }
```

### Deploy the contracts

Deploy each contract. Chain name should match the key of an object in the `chains` section of the config. Chain name should be omitted for contracts that are not chain specific.
Contract wasm binary can be passed by specifiying the path to the directory containing contract artifact files or by specifying the contract version. The contract version has to a be a tagged release in semantic version format vX.Y.Z or a commit hash.

- `ts-node cosmwasm/contract [store-code|instantiate|store-instantiate] -c [contract name] --artifact-dir [contract wasm dir] -e [environment] -n <chain name>`

- `ts-node cosmwasm/contract [store-code|instantiate|store-instantiate] -c [contract name] -v [contract version] -e [environment] -n <chain name>`

**Common options:**

- `-u, --rpc <axelarNode>`: Override the Axelar RPC URL from the config. Can also be set via the `AXELAR_RPC` environment variable.

Available subcommands:

- `store-code`: Uploads wasm file and saves codeId to `lastUploadedCodeId` in config

- `instantiate`: Instantiates a contract, it gets the codeId by order of priority from:
    1. Value of `--codeId` option
    2. From the network when using `--fetchCodeId` option by comparing previously uploaded bytecode's code hash with config `storeCodeProposalCodeHash`
    3. Value of previously saved `lastUploadedCodeId` in config
    - Use `--predictOnly` with `--instantiate2` to predict and save the address without instantiating

- `store-instantiate`: Both uploads and then instantiates a contract using the code Id that was just created. It doesn't accept `--codeId` nor `--fetchCodeId` options
    - Note: `instantiate2` is not supported with `--governance` flag for this command

- `migrate`: Migrates a contract using a new codeId, which is retrieved the same way as `instantiate` subcommand. The migrate message must be provided using the `--msg` option.

Some of the contracts depend on each other and need to be deployed in a specific order. Note the connection router and axelarnet gateway each need to know the other's address, so you need to pass `--instantiate2`, and upload both contract before instatiating them.

Example deployments with order dependency:

1.  `ts-node cosmwasm/contract store-code -c AxelarnetGateway --artifact-dir [contract wasm dir] --instantiate2 -e devnet`
2.  `ts-node cosmwasm/contract store-code -c Router --artifact-dir [contract wasm dir] --instantiate2 -e devnet`
3.  `ts-node cosmwasm/contract instantiate -c AxelarnetGateway --instantiate2 -e devnet`
4.  `ts-node cosmwasm/contract instantiate -c Router --instantiate2 -e devnet`
5.  `ts-node cosmwasm/contract store-instantiate -c ServiceRegistry --artifact-dir [contract wasm dir] -e devnet`
6.  `ts-node cosmwasm/contract store-instantiate -c Rewards --artifact-dir [contract wasm dir] -e devnet`
7.  `ts-node cosmwasm/contract store-instantiate -c Coordinator --artifact-dir [contract wasm dir] -e devnet`
8.  `ts-node cosmwasm/contract store-instantiate -c Multisig --artifact-dir [contract wasm dir] -e devnet`
9.  `ts-node cosmwasm/contract store-instantiate -c InterchainTokenService --artifact-dir [contract wasm dir] -e devnet`
10. `ts-node cosmwasm/contract store-instantiate -c VotingVerifier --artifact-dir [contract wasm dir] -e devnet -n avalanche`
11. `ts-node cosmwasm/contract store-instantiate -c Gateway --artifact-dir [contract wasm dir] -e devnet -n avalanche`
12. `ts-node cosmwasm/contract store-instantiate -c MultisigProver --artifact-dir [contract wasm dir] -e devnet -n avalanche`

### Constant Address Deployment

To deploy with a constant address using instantiate2, pass the `--instantiate2` flag.
To upload the contract and compute the expected address without instantiating, pass `--instantiate2` while using the `store-code` command. This will write the contract address and the code id to the config file.
A salt can be passed with `-s`. If no salt is passed but a salt is needed for constant address deployment, the contract name will be used as a salt.

Example:

```
ts-node cosmwasm/contract store-code -c Gateway --instantiate2 -s my-salt
```

### Deploying through governance proposals

On networks where only governance is allowed to upload bytecode or instantiate, the script `submit-proposal` can be used to submit a governance proposal.

```
ts-node submit-proposal.js <command> -m <mnemonic> -e <environment> -t <proposal title> -d <proposal description> --deposit <deposit> [options]
```

**Common options:**

- `-u, --rpc <axelarNode>`: Override the Axelar RPC URL from the config. Can also be set via the `AXELAR_RPC` environment variable.
- `--standardProposal`: Submit as a standard proposal instead of expedited (default is expedited). Use this flag if you want to use the standard proposal deposit amount and voting period.

**Expedited Proposals:**

By default, all governance proposals are submitted as expedited proposals, which have:

- A higher deposit requirement (configured via `govProposalExpeditedDepositAmount` in the config)
- A shorter voting period
- Faster execution after passing

The deposit amount is automatically set from the config based on whether the proposal is expedited (default) or standard (when `--standardProposal` flag is used). You can override the deposit amount by explicitly providing the `--deposit` option.

### Uploading bytecode through governance

Example usage:

```
ts-node cosmwasm/contract store-code -c ServiceRegistry --governance
```

For multiple contracts in a single proposal:

```
ts-node cosmwasm/contract store-code -c Gateway -c VotingVerifier -c MultisigProver --governance
```

By default, only governance will be able to instantiate the bytecode. To allow other addresses to instantiate the bytecode, pass `--instantiateAddresses [address1],[address2],[addressN]`.

For transparency and security, it's strongly recommended to include the `--source` and `--builder` options in your proposal:

- `--source`: Code Source URL is a valid absolute HTTPS URI to the contract's source code.
- `--builder`: Builder is a valid docker image name with tag, such as "cosmwasm/workspace-optimizer-arm64:0.16.0"

These options enable voters to independently verify that the proposed bytecode matches the public source code. For example: `--source "https://github.com/axelarnetwork/axelar-amplifier/tree/service-registry-v0.4.1/contracts/service-registry" --builder "cosmwasm/workspace-optimizer-arm64:0.16.0"`

After a store code proposal is accepted, the code id can be retrieved using the command `axelard q wasm list-code`

### Instantiating through governance

Prerequisites: Submit a proposal to upload the bytecode as described in the previous section and update `codeId` in the json config manually. TODO: create a script to automate this process.

Example usage:

```
ts-node cosmwasm/contract instantiate -c ServiceRegistry --fetchCodeId --governance
```

Use the option `--fetchCodeId` to retrieve and update the code id from the network by comparing the code hash of the uploaded bytecode with the code hash submitted through the store code proposal mentioned in the previous section.

Note: The rules for chain name specification and the use of `--instantiate2` as described in the "Deploy the contracts" and "Constant Address Deployment" sections above also apply when instantiating through governance. Refer to those sections for details on omitting chain names for certain contracts and using `--instantiate2` for address prediction.

Since the instantiation is not executed until the porposal passes, the contract address cannot be known in advance and therefore it cannot be saved in the config, unless the address is predicted using the `--instantiate2` flag.

Note: `-t` & `-d` is autogenrated, be can still be provided if required

Order of execution to satisfy dependencies:

1.  `ts-node cosmwasm/submit-proposal.js instantiate -c Router --instantiate2 --predictOnly`
2.  `ts-node cosmwasm/submit-proposal.js instantiate -c NexusGateway --instantiate2 --predictOnly`
3.  `ts-node cosmwasm/submit-proposal.js instantiate -c NexusGateway --instantiate2  --fetchCodeId -y`
4.  `ts-node cosmwasm/submit-proposal.js instantiate -c Router --instantiate2  --fetchCodeId -y`
5.  `ts-node cosmwasm/submit-proposal.js instantiate -c ServiceRegistry --instantiate2  --fetchCodeId -y`
6.  `ts-node cosmwasm/submit-proposal.js instantiate -c Rewards --instantiate2  --fetchCodeId -y`
7.  `ts-node cosmwasm/submit-proposal.js instantiate -c Coordinator --instantiate2  --fetchCodeId -y`
8.  `ts-node cosmwasm/submit-proposal.js instantiate -c Multisig --instantiate2  --fetchCodeId -y`
9.  `ts-node cosmwasm/submit-proposal.js instantiate -c VotingVerifier --instantiate2  --fetchCodeId -y -n "avalanche"`
10. `ts-node cosmwasm/submit-proposal.js instantiate -c Gateway --instantiate2  --fetchCodeId -y -n "avalanche"`
11. `ts-node cosmwasm/submit-proposal.js instantiate -c MultisigProver --instantiate2  --fetchCodeId -y -n "avalanche"`

### Instantiating chain contracts via Coordinator

Note: For new deployments, use the Coordinator contract to instantiate Gateway, VotingVerifier, and MultisigProver contracts together.

Use the `instantiate-chain-contracts` command:

```bash
ts-node cosmwasm/contract.ts instantiate-chain-contracts \
  -n avalanche \
  -s "salt123" \
  --fetchCodeId
```

This formats the execute message using the config, generates a deployment name (`<chain>-<codeId>`), and either executes directly (if using governance key i.e. devnet-amplifier) or submits a proposal to deploy all three contracts.

After the proposal executes, retrieve and write the deployed contract addresses to the config:

```bash
ts-node cosmwasm/query.js save-deployed-contracts avalanche
```

### Uploading and instantiating in one step

The command `store-instantiate` allows uploading and instantiating in one step. However, there are a couple of caveats to be aware of when using with governance:

1. There is no support for `instantiate2` using this proposal type with `--governance`. This means that the contract address will not be known until the proposal is executed and therefore it cannot be saved in the config.

2. Since governance proposals are executed asynchronously, both the codeId and contract address are not immediately available. Querying the network for the correct values could be tricky if multiple proposals are executed together.

Example usage:

Direct execution:

```
ts-node cosmwasm/contract store-instantiate -c ServiceRegistry --artifact-dir [contract wasm dir] -e devnet
```

Governance proposal:

```
ts-node cosmwasm/contract store-instantiate -c ServiceRegistry --governance
```

### Execute a contract through governance proposal

To submit a governance proposal to execute a contract, use the `submit-proposal` script with the `execute` command. The `--msg` option should be used to pass the execute message.

Example usage:

```
ts-node cosmwasm/submit-proposal.js execute -c Router -t "Proposal title" -d "Proposal description" --deposit 100000000 --msg '{"register_chain":{"chain":"avalanche","gateway_address":"axelar17cnq5hujmkf2lr2c5hatqmhzlvwm365rqc5ugryphxeftavjef9q89zxvp","msg_id_format":"hex_tx_hash_and_event_index"}}'
```

### Register or update chain on ITS Hub through governance proposal

To register an ITS chain, use the `contract` script with the `its-hub-register-chains <chains...>` command. The `chains` argument is used to pass a list of chains to register on ITS hub.

To update an existing chain registration (e.g., to change the translator contract), use the `its-hub-update-chains <chains...>` command.

**Prerequisites**: ITS hub contract configuration in json file must include the following attributes per chain:

| Attribute                   | Description                                                                                | Required | EVM | Sui |
| --------------------------- | ------------------------------------------------------------------------------------------ | -------- | --- | --- |
| `maxUintBits`               | Number of bits for the chain's maximum uint representation                                 | Yes      | 256 | 64  |
| `maxDecimalsWhenTruncating` | Maximum decimal places allowed when truncating ITS token amounts transferred to this chain | Yes      | 255 | 6   |
| `msgTranslator`             | Address of the message translator contract (defaults to global `ItsAbiTranslator.address`) | No       | -   | -   |

For EVM chains, `maxUintBits` and `maxDecimalsWhenTruncating` are used by default if not specified explicitly.

The ITS edge contract address must be configured in the chain's `contracts.InterchainTokenService` section (either `.address` for EVM chains or `.objects.ChannelId` for Sui chains).

The message translator address defaults to the global `ItsAbiTranslator.address` if not specified per-chain. To use a different translator for a specific chain, add `msgTranslator` to the chain's configuration.

Example configuration:

```json
"axelar": {
  "contracts": {
    ...
    "InterchainTokenService": {
      "address": "axelar1...",  // ITS Hub address
      ...
      "some-sui-chain": {
        "maxUintBits": 64,
        "maxDecimalsWhenTruncating": 6,
        "msgTranslator": "axelar1..."  // Optional: per-chain override
      },
      "some-evm-chain": {
        "maxUintBits": 256,  // Optional: defaults to 256 for EVM
        "maxDecimalsWhenTruncating": 255  // Optional: defaults to 255 for EVM
      }
    },
    "ItsAbiTranslator": {
      "address": "axelar1..."  // Global default for msgTranslator
    }
    ...
  }
}
```

Example usage:

```
# Register new chains
ts-node cosmwasm/contract.ts its-hub-register-chains avalanche-fuji sui-test2 -t "Proposal title" -d "Proposal description" --deposit 100000000

# Update existing chain registration (e.g., to change translator contract)
ts-node cosmwasm/contract.ts its-hub-update-chains aleo-2 -t "Update aleo-2 translator contract" -d "Update aleo-2 translator contract on ITS Hub" --deposit 100000000
```

### Submit a proposal to migrate a contract

To submit a governance proposal to migrate a contract, use the `submit-proposal` script with the `migrate` command. The `--msg` option should be used to pass the migrate message.

Note:

1. `-t` & `-d` is autogenrated, be can still be provided if required
2. `  --deposit` is automatically assigned from config baseed on `env` being used

Example usage:

```
ts-node cosmwasm/submit-proposal.js migrate \
  -c MultisigProver \
  -n avalanche \
  --msg '{}' \
  --fetchCodeId \
```

### Save chain contracts deployed via Coordinator

Query and save deployed contracts via Coordinator:

```bash
ts-node cosmwasm/query.js save-deployed-contracts <chain-name>
```

This will query the Coordinator contract for a specific chain's deployed addresses and update the config with those addresses.

### Rotating verifier set

1. Create a .env for rotation, containing mnemonic for multisig-prover-admin and environment:

```
MNEMONIC="<mnemonic for multisig prover admin>"
ENV="<environment>"
```

2. Register/Deregister verifiers for the chain in scope:

```bash
ampd register-chain-support <service-name> <chain-name>
or
ampd deregister-chain-support <service-name> <chain-name>
```

3. Update verifier set

```bash
ts-node cosmwasm/rotate-signers.js update-verifier-set <chain-name>
```

4. Using multisig session id output in last command, submit proof on destination chain. For example:

- Sui:

```bash
ts-node sui/gateway.js submitProof <multisig-session-id>
```

- EVM:

```bash
ts-node evm/gateway.js --action submitProof --multisigSessionId <multisig-session-id> -n <chain-name>
```

4. Confirm verifier rotation

```bash
ts-node cosmwasm/rotate-signers.js confirm-verifier-rotation <chain-name> <rotate-signers-tx>
```

### Querying Contract State

The `query.js` script provides commands to query various contract states and configurations. Use these commands to inspect contract information and verify deployments.

**Common options:**

- `-u, --rpc <axelarNode>`: Override the Axelar RPC URL from the config. Can also be set via the `AXELAR_RPC` environment variable.

#### Available Commands

##### Query Rewards Pool State

Query the rewards pool state for multisig and voting verifier contracts:

```bash
ts-node cosmwasm/query.js rewards <chainName>
```

##### Query Token Configuration

Query token configuration from the ITS Hub:

```bash
ts-node cosmwasm/query.js token-config <tokenId>
```

##### Query Custom Token Metadata

Query custom token metadata by chain name and token address:

```bash
ts-node cosmwasm/query.js custom-token-metadata <chainName> <tokenAddress>
```

##### Query Token Instance

Query token instance information by chain name and token ID:

```bash
ts-node cosmwasm/query.js token-instance <chainName> <tokenId>
```

##### Query ITS Chain Configuration

Query ITS chain configuration for a specific chain:

```bash
ts-node cosmwasm/query.js its-chain-config <chainName>
```

#### Examples

```bash
# Query rewards for flow chain
ts-node cosmwasm/query.js rewards flow

# Query token config for a specific token ID
ts-node cosmwasm/query.js token-config 1234567890abcdef...

# Query custom token metadata
ts-node cosmwasm/query.js custom-token-metadata flow 0x742d35cc6460c0f692b8f7b6b0e1b7f9e0c9f9f9

# Query token instance
ts-node cosmwasm/query.js token-instance flow 1234567890abcdef...

# Query ITS chain configuration
ts-node cosmwasm/query.js its-chain-config flow
```
