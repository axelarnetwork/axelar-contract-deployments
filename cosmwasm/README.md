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
Contract wasm binary can be passed by specifiying the path to the contract artifact file or by specifying the contract version. The contract version has to a be a tagged release in semantic version format vX.Y.Z or a commit hash.

-   `ts-node deploy-contract.js [upload|instantiate|upload-instantiate|migrate] -m [mnemonic] --artifact-path [contract wasm path] -c [contract name] -e [environment] -n <chain name>`

-   `ts-node deploy-contract.js [upload|instantiate|upload-instantiate|migrate] -m [mnemonic] -v [contract version] -c [contract name] -e [environment] -n <chain name>`

Available subcommands:

-   `upload`: Uploads wasm file and saves codeId to `lastUploadedCodeId` in config

-   `instantiate`: Instantiates a contract, it gets the codeId by order of priority from:

    1. Value of `--codeId` option
    2. From the network when using `--fetchCodeId` option by comparing previously uploaded bytecode's code hash with config `storeCodeProposalCodeHash`
    3. Value of previously saved `lastUploadedCodeId` in config

-   `upload-instantiate`: Both uploads and then instantiates a contract using the code Id that was just created. It doesn't accept `--codeId` nor `--fetchCodeId` options

-   `migrate`: Migrates a contract using a new codeId, which is retrieved the same way as `instantiate` subcommand. The migrate message must be provided using the `--msg` option.

Some of the contracts depend on each other and need to be deployed in a specific order. Note the connection router and axelarnet gateway each need to know the other's address, so you need to pass `--instantiate2`, and upload both contract before instatiating them.

Example deployments with order dependency:

1.  `ts-node deploy-contract.js upload -m [mnemonic] --artifact-path [contract wasm path] -c "AxelarnetGateway" --instantiate2 -e devnet`
2.  `ts-node deploy-contract.js upload -m [mnemonic] --artifact-path [contract wasm path] -c "Router" --instantiate2 -e devnet`
3.  `ts-node deploy-contract.js instantiate -m [mnemonic] -c "AxelarnetGateway" --instantiate2 -e devnet`
4.  `ts-node deploy-contract.js instantiate -m [mnemonic] -c "Router" --instantiate2 -e devnet`
5.  `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "ServiceRegistry" -e devnet`
6.  `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "Rewards" -e devnet`
7.  `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "Coordinator" -e devnet`
8.  `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "Multisig" -e devnet`
9.  `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "InterchainTokenService" -e devnet`
10. `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "VotingVerifier" -e devnet -n "avalanche"`
11. `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "Gateway" -e devnet -n "avalanche"`
12. `ts-node deploy-contract.js upload-instantiate -m [mnemonic] --artifact-path [contract wasm path] -c "MultisigProver" -e devnet -n "avalanche"`

### Constant Address Deployment

To deploy with a constant address using instantiate2, pass the `--instantiate2` flag.
To upload the contract and compute the expected address without instantiating, pass `--instantiate2` while using the `upload` subcommand. This will write the contract address and the code id to the config file.
A salt can be passed with `-s`. If no salt is passed but a salt is needed for constant address deployment, the contract name will be used as a salt.

### Deploying through governance proposals

On networks where only governance is allowed to upload bytecode or instantiate, the script `submit-proposal` can be used to submit a governance proposal.

```
ts-node submit-proposal.js <command> -m <mnemonic> -e <environment> -t <proposal title> -d <proposal description> --deposit <deposit> [options]
```

### Uploading bytecode through governance

Example usage:

```
ts-node cosmwasm/submit-proposal.js store -c ServiceRegistry
```

By default, only governance will be able to instantiate the bytecode. To allow other addresses to instantiate the bytecode, pass `--instantiateAddresses [address1],[address2],[addressN]`.

For transparency and security, it's strongly recommended to include the `--source` and `--builder` options in your proposal:

-   `--source`: Code Source URL is a valid absolute HTTPS URI to the contract's source code.
-   `--builder`: Builder is a valid docker image name with tag, such as "cosmwasm/workspace-optimizer-arm64:0.16.0"

These options enable voters to independently verify that the proposed bytecode matches the public source code. For example: `--source "https://github.com/axelarnetwork/axelar-amplifier/tree/service-registry-v0.4.1/contracts/service-registry" --builder "cosmwasm/workspace-optimizer-arm64:0.16.0"`

After a store code proposal is accepted, the code id can be retrieved using the command `axelard q wasm list-code`

### Instantiating through governance

Prerequisites: Submit a proposal to upload the bytecode as described in the previous section and update `codeId` in the json config manually. TODO: create a script to automate this process.

Example usage:

```
ts-node cosmwasm/submit-proposal.js instantiate -c ServiceRegistry --fetchCodeId
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

### Uploading and instantiating in one step

The command `storeInstantiate` from the `submit-proposal` script, allows uploading and instantiating in one step. However, there are a couple of caveats to be aware of:

1. There is no support for `instantiate2` using this proposal type. This means that the contract address will not be known until the proposal is executed and therefore it cannot be saved in the config.

2. Since governance proposals are executed asynchronously, both the codeId and contract address are not immediately available. Querying the network for the correct values could be tricky if multiple proposals are executed together.

Example usage:

```
ts-node cosmwasm/submit-proposal.js storeInstantiate -c ServiceRegistry -t "ServiceRegistry proposal title" -d "ServiceRegistry proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000
```

### Execute a contract through governance proposal

To submit a governance proposal to execute a contract, use the `submit-proposal` script with the `execute` command. The `--msg` option should be used to pass the execute message.

Example usage:

```
ts-node cosmwasm/submit-proposal.js execute -c Router -t "Proposal title" -d "Proposal description" --deposit 100000000 --msg '{"register_chain":{"chain":"avalanche","gateway_address":"axelar17cnq5hujmkf2lr2c5hatqmhzlvwm365rqc5ugryphxeftavjef9q89zxvp","msg_id_format":"hex_tx_hash_and_event_index"}}'
```

### Register chain on ITS Hub through governance proposal

To submit a governance proposal to register an ITS chain, use the `submit-proposal` script with the `its-hub-register-chains <chains...>` command. The `chains` argument is used to pass a list of chains to register on ITS hub.

**Prerequisites**: ITS hub contract configuration in json file must include the following attributes per chain:

| Attribute                   | Description                                                                                | EVM | Sui |
| --------------------------- | ------------------------------------------------------------------------------------------ | --- | --- |
| `maxUintBits`               | Number of bits for the chain's maximum uint representation                                 | 256 | 64  |
| `maxDecimalsWhenTruncating` | Maximum decimal places allowed when truncating ITS token amounts transferred to this chain | 255 | 6   |

For EVM chains, the values above are used by default if not specified explicitly.

Example configuration:

```
"axelar": {
  "contracts": {
    ...
    "InterchainTokenService": {
      ...
      "some-sui-chain": {
        "maxUintBits": 64,
        "maxDecimalsWhenTruncating": 6,
      }
    }
    ...
  }
}
```

Example usage:

```
ts-node cosmwasm/submit-proposal.js its-hub-register-chains avalanche-fuji sui-test2 -t "Proposal title" -d "Proposal description" --deposit 100000000 -r $RUN_AS_ACCOUNT
```

### Submit a proposal to change a parameter

To submit a governance proposal to change a parameter, use the `submit-proposal` script with the `paramChange` command. The `--changes` option should be used to pass a JSON string representing an array of parameter changes.

Note: `-t` & `-d` is still required for `paramChange` & `execute` command

Example usage:

```
ts-node cosmwasm/submit-proposal.js paramChange \
	-t "Set Gateway at Nexus Module" \
	-d "Proposal to update nexus param gateway address." \
	--changes '[
  {
    "subspace": "nexus",
    "key": "gateway",
    "value": "'$GATEWAY_ADDRESS'"
  }
]'
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
