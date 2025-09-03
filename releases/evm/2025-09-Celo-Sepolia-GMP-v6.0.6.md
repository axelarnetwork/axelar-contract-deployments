# Celo Sepolia testnet GMP v6.0.6

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io> |
| **Deployment** | @AttissNgo <attiss@interoplabs.io> |

| **Network**  | **Deployment Status** | **Date** |
| ------------ | --------------------- | -------- |
| **Stagenet** | -                     | TBD      |
| **Testnet**  | -                     | TBD      |

- [Releases](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/releases/tag/v6.0.6)

## Background

Celo Sepolia will replace Alfajores testnet when Holesky sunsets in September 2025. This is the GMP v6.0.6 deployment of EVM compatible Amplifier Gateway contracts for Celo Sepolia in stagenet and testnet. Mainnet is not affected.

## Deployment

Create an `.env` config. Local environment variable `CHAIN` should be set to `celo-sepolia`.

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<stagenet|testnet>
CHAINS=celo-sepolia
```

An initial chain config needs to be added to `${ENV}.json` file under `CHAIN` key.

Update npm dependencies (including contracts)

```bash
npm ci
```

#### Stagenet / Testnet

```bash
"$CHAIN": {
    "name": "Celo-Sepolia",
    "axelarId": "celo-sepolia",
    "chainId": 11142220,
    "rpc": "https://rpc.ankr.com/celo_sepolia",
    "tokenSymbol": "CELO",
    "confirmations": 1,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 1,
    "chainType": "evm",
    "explorer": {
      "name": "Celo Sepolia Explorer",
      "url": "https://celo-sepolia.blockscout.com/"
    },
  "contracts": {}
  }
```

1. Fund the following addresses with native tokens on chain:

| Network      | Addresses                                                                                                                                                                              |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                                                                                                                           |
| **Testnet**  | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

2. Deploy `ConstAddrDeployer`:

- `stagenet` and `testnet` use the same contract address, so we only deploy on `testnet`.

| Network      | `deployer address`                           |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
| **Testnet**  | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

```bash
ts-node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json
```

3. Deploy `Create3Deployer`:

- `stagenet` and `testnet` use the same contract address, so we only deploy on `testnet`.

| Network      | `deployer address`                           |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

```bash
ts-node evm/deploy-contract.js -c Create3Deployer -m create2
```

4. Waste nonce, this step should only be performed on `stagenet` and `testnet`. To generate the same `AmplifierGateway` address as older EVM chains we need to waste 2 nonce on the deployer key.

```bash
ts-node evm/send-tokens.js -r 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --amount 0.0001 # burn nonce 0
ts-node evm/send-tokens.js -r 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --amount 0.0001 # burn nonce 1
```

Note that since we only get one chance with the official deployer key nonce, the entire deployment flow should be run from a test account first.

5. Deploy Gateway contract

| Network      | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
| ------------ | ---------------------- | ---------------- | -------------------------------------------- |
| **Stagenet** | `300`                  | `create`         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**  | `3600`                 | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |

```bash
ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay]
```

6. Deploy `Operators`

| Network      | `deployer address`                           |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**  | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |

```bash
ts-node evm/deploy-contract.js -c Operators -m create2
```

7. After deploying the Operators contract, register the following operators according to their environment

| Network      | `operators`                                                                                |
| ------------ | ------------------------------------------------------------------------------------------ |
| **Stagenet** | `0x7054acf1b2d01e33b86235458edf0046cc354293`, `0xf669ed1ebc608c48f58b6e290df566ede7fb1103` |
| **Testnet**  | `0x8f23e84c49624a22e8c252684129910509ade4e2`, `0x3b401fa00191acb03c24ebb7754fe35d34dd1abd` |

```bash
ts-node evm/operators.js --action addOperator --args $OPERATOR_ADDRESS
```

8. Deploy GasService (set the `AxelarGasService.collector` to `Operators` contract address in config, which you will receive at step 6)

| Network      | `deployer address`                           | `deployMethod` |
| ------------ | -------------------------------------------- | -------------- |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `create2`      |
| **Testnet**  | `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85` | `create`       |

```bash
OPERATORS=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.Operators.address" | tr -d '"')

ts-node evm/deploy-upgradable.js -c AxelarGasService -m [deployMethod] --args "{\"collector\": \"$OPERATORS\"}"
```

8. Transfer ownership for contracts

- 8.1 Transfer Operators ownership

| Network      | `OPERATORS_OWNER_ADDRESS`                    |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

```bash
ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $OPERATORS_OWNER_ADDRESS
```

- 8.2 Transfer AxelarGateway ownership (mainnet and testnet only)

| Network     | New Owner Address                            |
| ----------- | -------------------------------------------- |
| **Testnet** | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

```bash
ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05
```

- 8.3 Transfer AxelarGasService ownership (testnet only)

```bash
ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05
```

## Checklist

The following checks should be performed after the rollout

### EVM -> EVM GMP call with Celo-Sepolia as source

1. Send a GMP call

```bash
ts-node evm/gateway.js -n $CHAIN --action callContract --destinationChain [destination-chain] --destination 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --payload 0x1234
```

2. Route GMP call via Amplifier

- https://docs.axelar.dev/dev/amplifier/chain-integration/relay-messages

3. Submit proof with multisig session id

```bash
ts-node evm/gateway.js -n [destination-chain] --action submitProof --multisigSessionId [multisig session id]
```

4. Confirm whether the message is approved

```bash
ts-node evm/gateway.js -n [destination-chain] --action isContractCallApproved --commandID [command-id] --sourceChain $CHAIN --sourceAddress 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --destination 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --payloadHash [payload-hash]
```

### EVM -> EVM GMP call with Celo-Sepolia as destination

1. Send a GMP call

```bash
ts-node evm/gateway.js -n [destination-chain] --action callContract --destinationChain $CHAIN --destination 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --payload 0x1234
```

2. Route GMP call via Amplifier

- https://docs.axelar.dev/dev/amplifier/chain-integration/relay-messages

3.  Submit proof with multisig session id

```bash
ts-node evm/gateway.js -n $CHAIN --action submitProof --multisigSessionId [multisig session id]
```

4. Confirm whether the message is approved

```bash
ts-node evm/gateway.js -n $CHAIN --action isContractCallApproved --commandID [command-id] --sourceChain [destination-chain] --sourceAddress [source-address] --destination [destination-address] --payloadHash [payload-hash]
```
