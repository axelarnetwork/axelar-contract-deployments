# Memento ITS v2.2.0

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @yourGithubUsername <user@interoplabs.io> |
| **Deployment** | @yourGithubUsername <user@interoplabs.io> |

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

Create an `.env` config. Local environment variable `CHAIN` should be set to `memento`.

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
ts-node evm/deploy-its.js -s "v2.2.0 devnet-amplifier" -m create2 --proxySalt 'v2.2.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
ts-node evm/deploy-its.js -s "v2.2.0" -m create2 --proxySalt 'v2.2.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Set Memento as trusted chain on remote ITS contracts

Set Memento as trusted chain on all EVM chains

```bash
ts-node evm/its.js set-trusted-chains memento hub -n all
```

Set Memento as trusted chain on Sui

```bash
ts-node sui/its.js add-trusted-chains memento
```

Set Memento as trusted chain on Stellar

```bash
ts-node stellar/its.js add-trusted-chains memento
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n memento -y
```

- Verify the token manager proxy contract once an ITS token is deployed on Memento and then mark it as a proxy.

- Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `wallet` below with the deployer key
ts-node evm/interchainTokenFactory.js --action deployInterchainToken --minter [wallet] --name "test" --symbol "TST" --decimals [decimals] --initialSupply 10000 --salt "salt1234"

# Register token metadata. Ensure GMP call is executed
ts-node evm/its.js --action registerTokenMetadata --tokenAddress [tokenAddress]
```

- Run from one chain to link to the remote token

```bash
# Register source token. Record tokenId from output for next steps.
ts-node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress [tokenAddress] --tokenManagerType 4 --operator [wallet] --salt "salt1234"

# Link to remote token. Ensure GMP call is executed
ts-node evm/interchainTokenFactory.js --action linkToken --destinationChain chain2 --destinationTokenAddress [remote token address] --tokenManagerType 4 --linkParams "0x" --salt "salt1234"
```

- Fetch tokenManager address for deployed token on both chains

```bash
# Record tokenManager address from output for transferMintership
ts-node evm/its.js --action tokenManagerAddress --tokenId [tokenId]
```

- Run on both chains

```bash
# Transfer mintership for each token to the token manager
ts-node evm/its.js --action transferMintership --tokenAddress [tokenAddress] --minter [tokenManager]
```

- Interchain Transfer (both ways)

```bash
ts-node evm/its.js --action interchainTransfer --destinationChain chain2 --tokenId [tokenId] --destinationAddress [recipient] --amount 1 --gasValue 0
```
