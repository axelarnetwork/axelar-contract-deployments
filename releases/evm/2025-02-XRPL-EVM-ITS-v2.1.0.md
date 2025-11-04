## XRPL EVM Sidechain ITS v2.1.0

|                | **Owner**                                                                  |
| -------------- | -------------------------------------------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io>                                     |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @milapsheth <milap@interoplabs.io> |

| **Network**           | **Deployment Status** | **Date**   |
| --------------------- | --------------------- | ---------- |
| **Devnet Amplifier**  | -                     | TBD        |
| **Stagenet**          | -                     | TBD        |
| **Testnet** (staging) | Completed             | 2025-02-19 |
| **Testnet**           | Completed             | 2025-03-13 |
| **Mainnet**           | Completed             | 2025-05-05 |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v)

## Background

- This is the XRPL EVM sidechain ITS release.

## Deployment

Ensure that [XRPL EVM GMP](../evm/2025-02-XRPL-EVM-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci && npm run build
```

Create an `.env` config. Use `all` for `CHAINS` to run the cmd for every EVM chain, or set a specific chain. `CHAIN` should be set to `xrpl-evm`.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=xrpl-evm
```

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Devnet Amplifier

```bash
ts-node evm/deploy-its.js -s "v2.1.0 devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
ts-node evm/deploy-its.js -s "v2.1.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register xrpl-evm ITS on ITS Hub

Please refer to `$DEPOSIT_VALUE` and `$RUN_AS_ACCOUNT` from [XRPL EVM GMP Amplifier](../cosmwasm/2025-02-XRPL-EVM-GMP-v6.0.4.md).

```bash
ts-node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" \
    --deposit $DEPOSIT_VALUE \
    --runAs $RUN_AS_ACCOUNT
```

## Set XRPL EVM as trusted chain on remote ITS contracts

Set XRPL EVM as trusted chain on remote ITS contracts for EVM and non-EVM chains.

```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

## Link XRP token

- Register XRP token metadata with ITS Hub.

```bash
ts-node evm/its.js register-token-metadata 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
```

- Submit `linkToken` msg from XRPL to XRPL EVM with the XRP token address as the destination token address.

- Query the linked token manager address for the XRP token.

```bash
ts-node evm/its.js token-manager-address [tokenId]
```

- The XRP token mint permission should then be transferred to the token manager.

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n $CHAIN -y
```

- Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `wallet` below with the deployer key
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name "test" --symbol "TST" --decimals 6 --initialSupply 10000 --minter [minter-address] --chainNames $CHAIN --salt "salt1234"

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token --destinationChain [destination-chain] --chainNames $CHAIN --salt "salt1234" -y

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [tokenId] [recipient] 1 --gasValue 1000000000000000000 -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [tokenId] [destination-address] 1 --gasValue 1000000000000000000 -n [destination-chain]
```
