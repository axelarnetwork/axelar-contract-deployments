# Memento ITS v2.2.0

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @nbayindirli <noah@interoplabs.io> |
| **Deployment** | @nbayindirli <noah@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0)

## Background

- This is the Memento ITS release.

## Deployment

Ensure that [Memento GMP](../evm/2025-09-Memento-GMP-v6.0.6.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAIN=memento
```

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Devnet Amplifier

```bash
ts-node evm/deploy-its.js -s "v2.2.0 devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
ts-node evm/deploy-its.js -s "v2.2.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Set Memento as trusted chain on remote ITS contracts

### Note: Ensure that Memento is registered on ITS hub

Set Memento as trusted chain on all EVM chains

```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

Set Memento as trusted chain on Sui

```bash
ts-node sui/its.js add-trusted-chains $CHAIN
```

Set Memento as trusted chain on Stellar

```bash
ts-node stellar/its.js add-trusted-chains $CHAIN
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n $CHAIN -y
```

- Verify the token manager proxy contract once an ITS token is deployed on Memento and then mark it as a proxy.

- EVM Checklist

```bash
# Create a token on Memento
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name test --symbol TST --decimals 6 --initialSupply 10000 --minter [minter-address] --chainNames $CHAIN --env <env> --salt salt1234

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token --destinationChain [destination-chain] --chainNames $CHAIN --env <env> --salt "salt1234" -y

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [token-id] [recipient] 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [token-id] [destination-address] 1 --gasValue [gas-value] -n [destination-chain]
```
