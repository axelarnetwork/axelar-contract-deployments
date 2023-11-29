### Cosmwasm deployment scripts

This folder contains deployment scripts for cosmwasm contracts needed for amplifier.

### Setup

`npm ci`

### Deploy the contracts

1. Compile the contracts in the amplifier [repo](https://github.com/axelarnetwork/axelar-amplifier) using the [rust optimizer](https://github.com/CosmWasm/rust-optimizer) for cosmwasm.

2. Add a `contracts` object to the `axelar` section of your config. Change any values as necessary. For chain specific contracts (`VotingVerifier`,`Gateway`,`MultisigProver`), there should be one object per chain, where the key is the chain id.
```
  "axelar": {
    "contracts": {
      "ServiceRegistry": {
        "governanceAccount": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
      },
      "ConnectionRouter": {
        "adminAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
        "governanceAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
      },
      "Multisig": {
        "governanceAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
        "gracePeriod": 5
      },
      "Rewards": {
        "governanceAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
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
          "serviceName": "validators",
          "sourceGatewayAddress": "0xe432150cce91c13a887f7D836923d5597adD8E31",
          "blockExpiry": 10,
          "confirmationHeight": 1,
          "votingThreshold": [
            "9",
            "10"
          ]
        }
      },
      "Gateway": {
        "ethereum": {}
      },
      "MultisigProver": {
        "ethereum-2": {
          "adminAddress": "axelar1gtm0wr3gpkzwgpjujzlyxvgj7a5ltcku99fdcz",
          "destinationChainID": "0",
          "signingThreshold": [
            "3",
            "5"
          ],
          "serviceName": "validators",
          "workerSetDiffThreshold": 1,
          "encoder": "abi",
          "keyType": "ecdsa"
        }
      }
    },
```
3. Deploy each contract. Chain name should match the key of an object in the `chains` section of the config. Chain name should be omitted for contracts that are not chain specific.

    `node deploy-contract.js -m [mnemonic] -a [path to contract artifacts] -c [contract name] -e [environment] -n <chain name>` 

### Deployment Order

The contracts depend on each other and will need to be deployed in a specific order. Below is one order that works:
1. ServiceRegistry, Rewards, ConnectionRouter (order between these 3 doesn't matter)
2. Multisig
3. VotingVerifier
4. Gateway
5. MultisigProver
