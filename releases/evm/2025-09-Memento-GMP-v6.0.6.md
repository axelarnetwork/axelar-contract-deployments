# Memento GMP v6.0.6

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @nbayindirli <noah@interoplabs.io> |
| **Deployment** | @nbayindirli <noah@interoplabs.io> |

| **Network**          | **Deployment Status** |  **Date**  |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-09-10 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

- [Releases](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/releases/tag/v6.0.6)

## Background

Changes in the release:

This is the v6.0.6 deployment of EVM compatible Amplifier Gateway contracts for Memento.

## Deployment

Create an `.env` config. Local environment variable `CHAIN` should be set to `memento`.

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAIN=memento
TESTNET_RPC_URL=<testnet rpc url>
MAINNET_RPC_URL=<mainnet rpc url>
```

An initial chain config needs to be added to `${ENV}.json` file under memento key.

Update npm dependencies (including contracts)

```bash
npm ci && npm run build
```

### Devnet-Amplifier / Stagenet / Testnet

```bash
"$CHAIN": {
    "name": "Memento",
    "axelarId": "$CHAIN",
    "chainId": 2129,
    "rpc": "https://private-rpc.memento.zeeve.online/35awf1GSKkyVgonq2lUn",
    "tokenSymbol": "ETH",
    "confirmations": 1,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 1,
    "chainType": "evm",
    "explorer": {
        "name": "Memento Testnet Explorer",
        "url": "https://explorer.memento.zeeve.online/"
    },
    "contracts": {}
  }
```

### Live network testing

Perform [Live network testing](https://github.com/axelarnetwork/axelar-cgp-solidity?tab=readme-ov-file#live-network-testing) in order to verify that the RPC endpoint is EVM-compatible and the Axelar gateway can be deployed on the external network. It is recommended to run the `RpcCompatibility` and `AxelarGateway` test groups.

### Mainnet

```bash
"$CHAIN": {
    "name": "Memento",
    "axelarId": "$CHAIN",
    "chainId": 51888,
    "rpc": "<$MAINNET_RPC_URL>",
    "tokenSymbol": "ETH",
    "confirmations": 1,
    "finality": "finalized",
    "decimals": 18,
    "approxFinalityWaitTime": 1,
    "chainType": "evm",
    "explorer": {
        "name": "Memento Explorer",
        "url": "https://priv-explorer.mementoblockchain.com/"
    },
    "contracts": {}
  }
```

### Steps

1. Fund the following addresses with native tokens on chain:

    | Network              | Addresses                                                                                                                                                                              |
    | -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
    | **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233`                                                                                                                                           |
    | **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                                                                                                                           |
    | **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
    | **Mainnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

1. Deploy `ConstAddrDeployer`:

    - `stagenet` and `testnet` use the same contract address, so we only deploy on `testnet`.

    | Network              | `deployer address`                           |
    | -------------------- | -------------------------------------------- |
    | **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
    | **Stagenet**         | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
    | **Testnet**          | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
    | **Mainnet**          | `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

    ```bash
    ts-node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath ../evm/legacy/ConstAddressDeployer.json -n $CHAIN
    ```

1. Deploy `Create3Deployer`:

    - `stagenet` and `testnet` use the same contract address, so we only deploy on `testnet`.

    | Network              | `deployer address`                           |
    | -------------------- | -------------------------------------------- |
    | **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
    | **Stagenet**         | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
    | **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
    | **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

    ```bash
    ts-node evm/deploy-contract.js -c Create3Deployer -m create2 -n $CHAIN
    ```

1. Waste nonce, this step should only be performed on `stagenet`, `testnet` and `mainnet`. To generate the same `AmplifierGateway` address as older EVM chains we need to waste 2 nonce on the deployer key.

    ```bash
    ts-node evm/send-tokens.js -r [deployer-address] --amount 0.0001 # burn nonce 0
    ts-node evm/send-tokens.js -r [deployer-address] --amount 0.0001 # burn nonce 1
    ```

    Note that since we only get one chance with the official deployer key nonce, the entire deployment flow should be run from a test account first.

1. Deploy Gateway contract

    | Network              | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
    | -------------------- | ---------------------- | ---------------- | -------------------------------------------- |
    | **Devnet-amplifier** | `0`                    | `create3`        | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
    | **Stagenet**         | `300`                  | `create`         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
    | **Testnet**          | `3600`                 | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
    | **Mainnet**          | `86400`                | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |

    ```bash
    ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay] -n $CHAIN
    ```

1. Deploy `Operators`

    | Network              | `deployer address`                           |
    | -------------------- | -------------------------------------------- |
    | **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
    | **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
    | **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
    | **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

    ```bash
    ts-node evm/deploy-contract.js -c Operators -m create2
    ```

1. After deploying the Operators contract, register the following operators according to their environment

    | Network              | `operatorAddresses`                                                                        |
    | -------------------- | ------------------------------------------------------------------------------------------ |
    | **Devnet-amplifier** | `0x01c793e1F8185a2527C5a2Ef3b4a3FBCb8982690`, `0xDb32E08fd5d6823E7f0298963E487d5df4e54b1E` |
    | **Stagenet**         | `0x7054acf1b2d01e33b86235458edf0046cc354293`, `0xf669ed1ebc608c48f58b6e290df566ede7fb1103` |
    | **Testnet**          | `0x8f23e84c49624a22e8c252684129910509ade4e2`, `0x3b401fa00191acb03c24ebb7754fe35d34dd1abd` |
    | **Mainnet**          | `0x0CDeE446bD3c2E0D11568eeDB859Aa7112BE657a`, `0x1a07a2Ee043Dd3922448CD53D20Aae88a67e486E` |

    ```bash
    ts-node evm/operators.js --action addOperator --args [operatorAddresses]
    ```

1. Deploy GasService (set the `AxelarGasService.collector` to `Operators` contract address in config, which you will receive at step 6)

    | Network              | `deployer address`                           | `deployMethod` |
    | -------------------- | -------------------------------------------- | -------------- |
    | **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `create2`      |
    | **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `create2`      |
    | **Testnet**          | `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85` | `create`       |
    | **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `create2`      |

    ```bash
    OPERATORS=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.Operators.address" | tr -d '"')

    ts-node evm/deploy-upgradable.js -c AxelarGasService -m [deployMethod] --args "{\"collector\": \"$OPERATORS\"}"
    ```

1. Transfer ownership for contracts

    1. Transfer Operators ownership

        | Network              | `OPERATORS_OWNER_ADDRESS`                    |
        | -------------------- | -------------------------------------------- |
        | **Devnet-amplifier** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` |
        | **Stagenet**         | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` |
        | **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

        ```bash
        ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $OPERATORS_OWNER_ADDRESS
        ```

    1. Transfer AxelarGateway ownership (mainnet and testnet only)

        | Network              | New Owner Address                            |
        | -------------------- | -------------------------------------------- |
        | **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
        | **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

        ```bash
        ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05
        ```

    1. Transfer AxelarGateway ownership (testnet only)

        ```bash
        ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05
        ```

## Checklist

The following checks should be performed after the rollout

### Memento -> EVM GMP call with Memento as source

1. Send a GMP call

    ```bash
    ts-node evm/gateway.js -n $CHAIN --action callContract --destinationChain [destination-chain] --destination [deployer-address] --payload 0x1234
    ```

1. Route GMP call via Amplifier

    - <https://docs.axelar.dev/dev/amplifier/chain-integration/relay-messages>

1. Submit proof with multisig session id

    ```bash
    ts-node evm/gateway.js -n [destination-chain] --action submitProof --multisigSessionId [multisig session id]
    ```

1. Confirm whether the message is approved

    ```bash
    ts-node evm/gateway.js -n [destination-chain] --action isContractCallApproved --commandID [command-id] --sourceChain $CHAIN --sourceAddress [deployer-address] --destination [deployer-address] --payloadHash [payload-hash]
    ```

### EVM -> Memento GMP call with Memento as destination

1. Send a GMP call

    ```bash
    ts-node evm/gateway.js -n [destination-chain] --action callContract --destinationChain $CHAIN --destination [deployer-address] --payload 0x1234
    ```

1. Route GMP call via Amplifier

    - <https://docs.axelar.dev/dev/amplifier/chain-integration/relay-messages>

1. Submit proof with multisig session id

    ```bash
    ts-node evm/gateway.js -n $CHAIN --action submitProof --multisigSessionId [multisig session id]
    ```

1. Confirm whether the message is approved

    ```bash
    ts-node evm/gateway.js -n $CHAIN --action isContractCallApproved --commandID [command-id] --sourceChain [destination-chain] --sourceAddress [source-address] --destination [destination-address] --payloadHash [payload-hash]
    ```
