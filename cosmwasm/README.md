### Cosmwasm deployment scripts

This folder contains deployment scripts for cosmwasm contracts needed for amplifier.

### Setup

`npm ci`

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

    `node deploy-contract.js -m [mnemonic] -a [path to contract artifacts] -c [contract name] -e [environment] -n <chain name>`

Some of the contracts depend on each other and need to be deployed in a specific order. Note the connection router and nexus gateway each need to know the other's address, so you need to pass `--instantiate2`, and upload each contract before instatiating (by passing `-u`).

1.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "Router" --instantiate2 -e devnet -u`
2.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "NexusGateway" --instantiate2 -e devnet -u`
3.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "NexusGateway" --instantiate2 -e devnet -r`
4.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "Router" --instantiate2 -e devnet -r`
5.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "ServiceRegistry" -e devnet`
6.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "Rewards" -e devnet`
7.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "Coordinator" -e devnet`
8.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "Multisig" -e devnet`
9.  `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "VotingVerifier" -e devnet -n "ethereum,avalanche"`
10. `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "Gateway" -e devnet -n "ethereum,avalanche"`
11. `node deploy-contract.js -m [mnemonic] -a [path to artifacts] -c "MultisigProver" -e devnet -n "ethereum,avalanche"`

### Constant Address Deployment

To deploy with a constant address using instantiate2, pass the `--instantiate2` flag.
To upload the contract and compute the expected address without instantiating, pass `--instantiate2` and `-u`. This will write the contract address and the code id to the config file.
A salt can be passed with `-s`. If no salt is passed but a salt is needed for constant address deployment, the contract name will be used as a salt.
Pass `-r` to skip the upload step, and reuse the previous code id (specified in the config).

### Deploying through governance proposals

On networks where only governance is allowed to upload bytecode or instantiate, the script `submit-proposal` can be used to submit a governance proposal.

```
node submit-proposal.js <command> -m <mnemonic> -e <environment> -t <proposal title> -d <proposal description> --deposit <deposit> [options]
```

### Uploading bytecode through governance

Example usage:

```
node cosmwasm/submit-proposal.js store -c ServiceRegistry -t "Proposal title" -d "Proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000
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
node cosmwasm/submit-proposal.js instantiate -c ServiceRegistry -t "Proposal title" -d "Proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000  --fetchCodeId
```

Use the option `--fetchCodeId` to retrieve and update the code id from the network by comparing the code hash of the uploaded bytecode with the code hash submitted through the store code proposal mentioned in the previous section.

Note: The rules for chain name specification and the use of `--instantiate2` as described in the "Deploy the contracts" and "Constant Address Deployment" sections above also apply when instantiating through governance. Refer to those sections for details on omitting chain names for certain contracts and using `--instantiate2` for address prediction.

Since the instantiation is not executed until the porposal passes, the contract address cannot be known in advance and therefore it cannot be saved in the config, unless the address is predicted using the `--instantiate2` flag.

Order of execution to satisfy dependencies:

1.  `node cosmwasm/submit-proposal.js instantiate -c Router -t "Router roposal title" -d "Router proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2 --predictOnly`
2.  `node cosmwasm/submit-proposal.js instantiate -c NexusGateway -t "NexusGateway roposal title" -d "NexusGateway proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2 --predictOnly`
3.  `node cosmwasm/submit-proposal.js instantiate -c NexusGateway -t "NexusGateway roposal title" -d "NexusGateway proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y`
4.  `node cosmwasm/submit-proposal.js instantiate -c Router -t "Router roposal title" -d "Router proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y`
5.  `node cosmwasm/submit-proposal.js instantiate -c ServiceRegistry -t "ServiceRegistry roposal title" -d "ServiceRegistry proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y`
6.  `node cosmwasm/submit-proposal.js instantiate -c Rewards -t "Rewards roposal title" -d "Rewards proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y`
7.  `node cosmwasm/submit-proposal.js instantiate -c Coordinator -t "Coordinator roposal title" -d "Coordinator proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y`
8.  `node cosmwasm/submit-proposal.js instantiate -c Multisig -t "Multisig roposal title" -d "Multisig proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y`
9.  `node cosmwasm/submit-proposal.js instantiate -c VotingVerifier -t "VotingVerifier roposal title" -d "VotingVerifier proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y -n "avalanche"`
10. `node cosmwasm/submit-proposal.js instantiate -c Gateway -t "Gateway roposal title" -d "Gateway proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y -n "avalanche"`
11. `node cosmwasm/submit-proposal.js instantiate -c MultisigProver -t "MultisigProver roposal title" -d "MultisigProver proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000 --instantiate2  --fetchCodeId -y -n "avalanche"`

### Uploading and instantiating in one step

The command `storeInstantiate` from the `submit-proposal` script, allows uploading and instantiating in one step. However, there are a couple of caveats to be aware of:

1. There is no support for `instantiate2` using this proposal type. This means that the contract address will not be known until the proposal is executed and therefore it cannot be saved in the config.

2. Since governance proposals are executed asynchronously, both the codeId and contract address are not immediately available. Querying the network for the correct values could be tricky if multiple proposals are executed together.

Example usage:

```
node cosmwasm/submit-proposal.js storeInstantiate -c ServiceRegistry -t "ServiceRegistry proposal title" -d "ServiceRegistry proposal description" -r $RUN_AS_ACCOUNT --deposit 100000000
```

### Execute a contract through governance proposal

To submit a governance proposal to execute a contract, use the `submit-proposal` script with the `execute` command. The `--msg` option should be used to pass the execute message.

Example usage:

```
node cosmwasm/submit-proposal.js execute -c Router -t "Proposal title" -d "Proposal description" --deposit 100000000 --msg '{"register_chain":{"chain":"avalanche","gateway_address":"axelar17cnq5hujmkf2lr2c5hatqmhzlvwm365rqc5ugryphxeftavjef9q89zxvp","msg_id_format":"hex_tx_hash_and_event_index"}}'
```

### Submit a proposal to change a parameter

To submit a governance proposal to change a parameter, use the `submit-proposal` script with the `paramChange` command. The `--changes` option should be used to pass a JSON string representing an array of parameter changes.

Example usage:

```
node cosmwasm/submit-proposal.js paramChange \
	-t "Set Gateway at Nexus Module" \
	-d "Proposal to update nexus param gateway address." \
	--deposit 100000000 \
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

Example usage:

```
node cosmwasm/submit-proposal.js migrate \
  -c MultisigProver \
  -t "Proposal title" \
  -d "Proposal description" \
  -n avalanche \
  --msg '{}' \
  --fetchCodeId \
  --deposit 100000000
```
