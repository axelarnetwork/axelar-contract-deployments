# Robinhood GMP

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

- [Releases](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/releases)

## Background

This is the deployment of EVM compatible Amplifier Gateway contracts for Robinhood Chain Testnet.

## Deployment

Ensure that [CosmWasm Robinhood GMP](../cosmwasm/2026-09-Robinhood-GMP.md) is deployed first.

Create an `.env` config. Set `CHAIN=robinhood` in your shell for the commands below.

```yaml
PRIVATE_KEY=<deployer private key>
ENV=testnet
CHAIN=robinhood
CHAINS=robinhood
```

Add the initial chain config under `chains.robinhood` in `axelar-chains-config/info/testnet.json`.

Update npm dependencies (including contracts)

```bash
npm ci && npm run build
```

### Testnet

```json
{
    "robinhood": {
        "name": "Robinhood Chain Testnet",
        "axelarId": "robinhood",
        "chainId": 46630,
        "rpc": "https://rpc.testnet.chain.robinhood.com",
        "explorer": {
            "name": "Blockscout",
            "url": "https://explorer.testnet.chain.robinhood.com"
        },
        "tokenSymbol": "ETH",
        "confirmations": 1,
        "finality": "finalized",
        "decimals": 18,
        "approxFinalityWaitTime": 25,
        "chainType": "evm",
        "contracts": {}
    }
}
```

### Live network testing

Perform [Live network testing](https://github.com/axelarnetwork/axelar-cgp-solidity?tab=readme-ov-file#live-network-testing) in order to verify that the RPC endpoint is EVM-compatible and the Axelar gateway can be deployed on the external network. It is recommended to run the `RpcCompatibility` and `AxelarGateway` test groups.

### Mainnet

Deployment parameters and date are TBD.

### Steps

1. Fund the following addresses with native tokens on chain:

    | Network     | Addresses                                    |
    | ----------- | -------------------------------------------- |
    | **Testnet** | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

1. Deploy `ConstAddressDeployer`:

    | Network     | `deployer address`                           |
    | ----------- | -------------------------------------------- |
    | **Testnet** | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

    ```bash
    ts-node evm/deploy-contract.js -c ConstAddressDeployer -m create --artifactPath evm/legacy/ConstAddressDeployer.json
    ```

1. Deploy `Create3Deployer`:

    | Network     | `deployer address`                           |
    | ----------- | -------------------------------------------- |
    | **Testnet** | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

    ```bash
    ts-node evm/deploy-contract.js -c Create3Deployer -m create2 -s "v1.0.13"
    ```

1. Deploy Gateway contract
    - **Note:** Switch to the Gateway deployer private key before this step.

    | Network     | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
    | ----------- | ---------------------- | ---------------- | -------------------------------------------- |
    | **Testnet** | `3600`                 | `create`         | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

    ```bash
    ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay]
    ```

1. Deploy `Operators`
    - **Note:** Use the same deployer as the Gateway (should already have the correct private key set).

    | Network     | `deployer address`                           |
    | ----------- | -------------------------------------------- |
    | **Testnet** | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

    ```bash
    ts-node evm/deploy-contract.js -c Operators -m create2 -s "v1.0.13"
    ```

1. After deploying the Operators contract, register the testnet relayer operator addresses. Replace `[operator-addresses]` with the addresses supplied by the relayer operators.

    | Network     | `operatorAddresses`    |
    | ----------- | ---------------------- |
    | **Testnet** | `[operator-addresses]` |

    ```bash
    ts-node evm/operators.js --action addOperator --args [operatorAddresses]
    ```

1. Deploy GasService (set the `AxelarGasService.collector` to `Operators` contract address in config, from the Operators deployment above)
    - **Note:** Switch to the GasService deployer private key before this step.

    | Network     | `deployer address`                           | `deployMethod` |
    | ----------- | -------------------------------------------- | -------------- |
    | **Testnet** | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` | `create`       |

    ```bash
    OPERATORS=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.Operators.address" | tr -d '"')

    ts-node evm/deploy-upgradable.js -c AxelarGasService -m [deployMethod] --args "{\"collector\": \"$OPERATORS\"}"
    ```

1. Transfer ownership for contracts
    - **Note:** Switch back to the Gateway deployer private key (the one that deployed Gateway and Operators) before this step.
    1. Transfer Operators ownership

        | Network     | `OPERATORS_OWNER_ADDRESS`                    |
        | ----------- | -------------------------------------------- |
        | **Testnet** | `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE` |

        ```bash
        ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $OPERATORS_OWNER_ADDRESS
        ```

    1. Transfer AxelarGateway ownership

        | Network     | New Owner Address                            |
        | ----------- | -------------------------------------------- |
        | **Testnet** | `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE` |

        ```bash
        ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner [NEW_OWNER_ADDRESS]
        ```

    1. Transfer AxelarGasService ownership

        | Network     | New Owner Address                            |
        | ----------- | -------------------------------------------- |
        | **Testnet** | `0x49845e5d9985d8dc941462293ed38EEfF18B0eAE` |

        ```bash
        ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner [NEW_OWNER_ADDRESS]
        ```

## Checklist

The following checks should be performed after the rollout

### Robinhood -> EVM GMP call with Robinhood as source

1. Send a GMP call

    ```bash
    ts-node evm/gateway.js -n $CHAIN --action callContract --destinationChain [destination-chain] --destination [destination-address] --payload 0x1234
    ```

1. Route GMP call via Amplifier
    - <https://docs.axelar.dev/dev/amplifier/chain-integration/relay-messages>

1. Submit proof with multisig session id

    ```bash
    ts-node evm/gateway.js -n [destination-chain] --action submitProof --multisigSessionId [multisig session id]
    ```

1. Confirm whether the message is approved

    ```bash
    ts-node evm/gateway.js -n [destination-chain] --action isContractCallApproved --commandID [command-id] --sourceChain $CHAIN --sourceAddress [source-address] --destination [destination-address] --payloadHash [payload-hash]
    ```

### EVM -> Robinhood GMP call with Robinhood as destination

1. Send a GMP call

    ```bash
    ts-node evm/gateway.js -n [source-chain] --action callContract --destinationChain $CHAIN --destination [destination-address] --payload 0x1234
    ```

1. Route GMP call via Amplifier
    - <https://docs.axelar.dev/dev/amplifier/chain-integration/relay-messages>

1. Submit proof with multisig session id

    ```bash
    ts-node evm/gateway.js -n $CHAIN --action submitProof --multisigSessionId [multisig session id]
    ```

1. Confirm whether the message is approved

    ```bash
    ts-node evm/gateway.js -n $CHAIN --action isContractCallApproved --commandID [command-id] --sourceChain [source-chain] --sourceAddress [source-address] --destination [destination-address] --payloadHash [payload-hash]
    ```
