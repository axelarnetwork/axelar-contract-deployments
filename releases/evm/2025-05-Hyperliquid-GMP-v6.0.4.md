# Hyperliquid GMP v6.0.4

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @isi8787 <isaac@interoplabs.io> |
| **Deployment** | @isi8787 <isaac@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Pending             | TBD |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

- [Releases](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/releases/tag/v6.0.4)

## Background

Changes in the release:

This is the v6.0.4 deployment of EVM compatible Amplifier Gateway contracts for Hyperliquid.

## Deployment

Create an `.env` config. Local environment variable `CHAIN` should be set to `hyperliquid`.

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
  "name": "Hyperliquid",
  "axelarId": "hyperliquid",
  "networkType": "testnet",
  "chainId": 998,
  "rpc": "https://rpc.hyperliquid-testnet.xyz/evm",
  "tokenSymbol": "HYPE",
  "confirmations": 1,
  "finality": "finalized",
  "decimals": 18,
  "approxFinalityWaitTime": 1,
  "chainType": "evm",
  "explorer": {
    "name": "Hyperliquid-testnet Explorer",
    "url": "https://app.hyperliquid-testnet.xyz/explorer"
  },
  "contracts": {}
}
```

#### Mainnet

```json
"$CHAIN": {
  "name": "Hyperliquid",
  "axelarId": "hyperliquid",
  "chainId": "999",
  "rpc": "https://rpc.hyperliquid.xyz/evm",
  "tokenSymbol": "HYPE",
  "confirmations": 1,
  "finality": "finalized",
  "decimals": 18,
  "approxFinalityWaitTime": 1,
  "chainType": "evm",
  "explorer": {
    "name": "Hyperliquid Explorer",
    "url": "https://app.hyperliquid.xyz/explorer"
  },
  "contracts": {}
}
```

Ensure python3 is installed on your system, recomended version is 3.10, but was tested succesfully with 3.13.   

1. Fund the following addresses with native tokens on chain:

| Network              | Addresses                                                                                                                                                                              |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233`                                                                                                                                           |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F`                                                                                                                                           |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |
| **Mainnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC`, `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85`, `0xE86375704CDb8491a5Ed82D90DceCE02Ee0ac25F` |

2. Set Deployer Keys to Use Slow/Big Blocks

Hyperliquid EVM uses a dual architecture block model:
- **Fast blocks**: 2 seconds duration with a 2M gas limit
- **Slow blocks**: 1 minute duration with a 30M gas limit

Contract deployments exceed the fast block gas limit and will require that each deployer key be permissioned to use the slow block model. Additional instructtions are provided if accounts needs to be converted back to fast block to utilize the faster finalization rate.

a. Clone the Hyperliquid Python SDK. This document was prepared using release v0.13 (commit hash: 583a96dc0af53c6d0b4eed06afb5a5c08481821d):
   ```bash
   git clone https://github.com/hyperliquid-dex/hyperliquid-python-sdk.git
   cd hyperliquid-python-sdk
   ```

b. Edit the `./hyperliquid-python-sdk/examples/basic_evm_use_big_blocks.py` file:
   #### For devnet-amplifier, testnet and stagenet
   ```bash
   address, info, exchange = example_utils.setup(constants.TESTNET_API_URL, skip_ws=True)
   ``` 

   #### For mainnet
   ```bash
   address, info, exchange = example_utils.setup(constants.MAINNET_API_URL, skip_ws=True)
   ``` 
   - Comment out or delete:
   ```bash
   print(exchange.use_big_blocks(False))
   ``` 

c. Fund one account with HYPE on both HyperCore and Hyperliquid EVM. Steps to procure and swap funds are:
   #### For devnet-amplifier, testnet and stagenet
    - Provision USDC funds: from their faucet at: https://app.hyperliquid-testnet.xyz/drip. Faucet requires account exist on mainnet.
    - Use their trading app https://app.hyperliquid-testnet.xyz/trade and connect wallet.
    - Buy HYPE with USDC balance
    - Under `balances` section connect wallet again to perform an EVM transfer.

     #### For mainnet
    - Provision USDC on Arbitrum
    - Use their trading app https://app.hyperliquid.xyz/trade and connect wallet.
    - Buy HYPE with USDC balance
    - Under `balances` section connect wallet again to perform an EVM transfer.

    Note: Above flow has been tested. In order to preserve nonces do not transfer funds from EVM to Hypercore

d. Update the `./hyperliquid-python-sdk/examples/config.json`:

- Set the main funded account as the secret_key
- Set the deployer address as the account_address

e. Run the script:
```bash
python3 examples/basic_evm_use_big_blocks.py
```

Steps `c`, `d` and `e` needs to be repeated for each deployer key.  

f. Delete private key information from `./hyperliquid-python-sdk/examples/config.json`

After release is complete the deployer keys can set to utilize fast blocks again to enable faster operations that dont require larger gas limits of slow blocks. To disable slow/big blocks Edit the `./hyperliquid-python-sdk/examples/basic_evm_use_big_blocks.py` file to add back `print(exchange.use_big_blocks(True))` and rerun step `d` and `e`


3. Deploy `ConstAddrDeployer`:

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

4. Deploy `Create3Deployer`:

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

5. Waste nonce, this step should only be performed on `stagenet`, `testnet` and `mainnet`. To generate the same `AmplifierGateway` address as older EVM chains we need to waste 2 nonce on the deployer key.

```bash
ts-node evm/send-tokens.js -r 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --amount 0.0001 # burn nonce 0
ts-node evm/send-tokens.js -r 0xba76c6980428A0b10CFC5d8ccb61949677A61233 --amount 0.0001 # burn nonce 1
```

6. Deploy Gateway contract

| Network              | `minimumRotationDelay` | `deploymentType` | `deployer`                                   |
| -------------------- | ---------------------- | ---------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0`                    | `create3`        | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `300`                  | `create`         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `3600`                 | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `86400`                | `create`         | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |

```bash
ts-node evm/deploy-amplifier-gateway.js -m [deploymentType] --minimumRotationDelay [minimumRotationDelay]
```

7. Deploy `Operators`

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

```bash
ts-node evm/deploy-contract.js -c Operators -m create2
```

8. After deploying the Operators contract, register the following operators according to their environment

| Network              | `operators`                                                                                |
| -------------------- | ------------------------------------------------------------------------------------------ |
| **Devnet-amplifier** | ?                                                                                          |
| **Stagenet**         | `0x7054acf1b2d01e33b86235458edf0046cc354293`, `0xf669ed1ebc608c48f58b6e290df566ede7fb1103` |
| **Testnet**          | `0x8f23e84c49624a22e8c252684129910509ade4e2`, `0x3b401fa00191acb03c24ebb7754fe35d34dd1abd` |
| **Mainnet**          | `0x0CDeE446bD3c2E0D11568eeDB859Aa7112BE657a`, `0x1a07a2Ee043Dd3922448CD53D20Aae88a67e486E` |

```bash
ts-node evm/operators.js --action addOperator --args $OPERATOR_ADDRESS
```

9. Deploy GasService (set the `AxelarGasService.collector` to `Operators` address in config, which you will receive at step 7)

| Network              | `deployer address`                           | `deployMethod` |
| -------------------- | -------------------------------------------- | -------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `create2`      |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `create2`      |
| **Testnet**          | `0x5b593E7b1725dc6FcbbFe80b2415B19153F94A85` | `create`       |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `create2`      |

```bash
ts-node evm/deploy-upgradable.js -c AxelarGasService -m [deployMethod] --args '{"collector": "$OPERATOR_ADDRESS"}'
```

10. Transfer ownerships for gateway, operators and gas service contracts on `mainnet` and `testnet`

```bash
# Only for mainnet and official testnet connection
ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner 0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05
```

## Checklist

The following checks should be performed after the rollout

### Hyperliquid -> EVM GMP call

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

### EVM -> Hyperliquid GMP Call

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
