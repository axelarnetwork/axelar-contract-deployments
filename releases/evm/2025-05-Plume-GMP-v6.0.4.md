# Plume GMP v6.0.4

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io> |
| **Deployment** | @AttissNgo <attiss@interoplabs.io>, @milapsheth <milap@interoplabs.io>                                   |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-04-30 |
| **Stagenet**         | Completed             | 2025-05-07 |
| **Testnet**          | Completed             | 2025-05-27 |
| **Mainnet**          | Completed             | 2025-06-11 |

- [Releases](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/releases/tag/v6.0.4)

## Background

Changes in the release:

This is the v6.0.4 deployment of EVM compatible Amplifier Gateway contracts for Plume.

## Deployment

Create an `.env` config. Local environment variable `CHAIN` should be set to `plume`.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=xyz
```

An initial chain config needs to be added to `${ENV}.json`.

Update npm dependencies (including contracts)

```bash
npm ci
```

#### Devnet-Amplifier / Stagenet / Testnet

```json
"$CHAIN": {
  "name": "Plume",
  "axelarId": "plume",
  "networkType": "testnet",
  "chainId": 98867,
  "rpc": "https://testnet-rpc.plumenetwork.xyz",
  "tokenSymbol": "PLUME",
  "confirmations": 1,
  "finality": "finalized",
  "decimals": 18,
  "approxFinalityWaitTime": 53,
  "chainType": "evm",
  "explorer": {
    "name": "Plume-testnet Explorer",
    "url": "https://testnet-explorer.plumenetwork.xyz/"
  },
  "contracts": {}
}
```

#### Mainnet

```json
"$CHAIN": {
  "name": "Plume",
  "axelarId": "plume",
  "chainId": "TBD",
  "rpc": "TBD",
  "tokenSymbol": "PLUME",
  "confirmations": 1,
  "finality": "finalized",
  "decimals": 18,
  "approxFinalityWaitTime": 30,
  "chainType": "evm",
  "explorer": {
    "name": "TBD",
    "url": "TBD",
    "api": "TBD"
  },
  "contracts": {}
}
```

1. Fund the following addresses with native tokens on chain:

| Network              | Addresses                                                                                                                                                                              |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233`                                                                                                                                           |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                                                                                                                           |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
| **Mainnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

2. Deploy `ConstAddrDeployer`:

- `stagenet` and `testnet` use the same contract address, so we only deploy on `testnet`.

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
| **Testnet**          | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
| **Mainnet**          | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

```bash
ts-node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json
```

3. Deploy `Create3Deployer`:

- `stagenet` and `testnet` use the same contract address, so we only deploy on `testnet`.

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

```bash
ts-node evm/deploy-contract.js -c Create3Deployer -m create2
```

4. Waste nonce, this step should only be performed on `stagenet`, `testnet` and `mainnet`. To generate the same `AmplifierGateway` address as older EVM chains we need to waste 2 nonce on the deployer key.

```bash
ts-node evm/send-tokens.js -r 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --amount 0.0001 # burn nonce 0
ts-node evm/send-tokens.js -r 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --amount 0.0001 # burn nonce 1
```

5. Deploy Gateway contract

| Network              | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
| -------------------- | ---------------------- | ---------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0`                    | `create3`        | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `300`                  | `create`         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `3600`                 | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `86400`                | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |

```bash
ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay]
```

6. Deploy `Operators`

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

```bash
ts-node evm/deploy-contract.js -c Operators -m create2
```

7. After deploying the Operators contract, register the following operators according to their environment

| Network              | `operators`                                                                                |
| -------------------- | ------------------------------------------------------------------------------------------ |
| **Devnet-amplifier** | ?                                                                                          |
| **Stagenet**         | `0x7054acf1b2d01e33b86235458edf0046cc354293`, `0xf669ed1ebc608c48f58b6e290df566ede7fb1103` |
| **Testnet**          | `0x8f23e84c49624a22e8c252684129910509ade4e2`, `0x3b401fa00191acb03c24ebb7754fe35d34dd1abd` |
| **Mainnet**          | `0x0CDeE446bD3c2E0D11568eeDB859Aa7112BE657a`, `0x1a07a2Ee043Dd3922448CD53D20Aae88a67e486E` |

```bash
ts-node evm/operators.js --action addOperator --args $OPERATOR_ADDRESS
```

8. Deploy GasService (set the `AxelarGasService.collector` to `Operators` address in config, which you will receive at step 6)

| Network              | `deployer address`                           | `deployMethod` |
| -------------------- | -------------------------------------------- | -------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `create2`      |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `create2`      |
| **Testnet**          | `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85` | `create`       |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `create2`      |

```bash
ts-node evm/deploy-upgradable.js -c AxelarGasService -m [deployMethod] --args '{"collector": "$OPERATOR_ADDRESS"}'
```

8. Transfer ownerships for gateway, operators and gas service contracts on `mainnet` and `testnet`

```bash
# For mainnet
ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05

# For testnet
ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05

ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05
```

## Checklist

The following checks should be performed after the rollout

### Plume -> EVM GMP call

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

### EVM -> Plume GMP Call

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
