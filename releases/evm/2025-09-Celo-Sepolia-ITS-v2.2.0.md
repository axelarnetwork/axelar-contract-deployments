# Celo-Sepolia ITS v2.2.0

|                | **Owner**                                                              |
| -------------- | ---------------------------------------------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io>                                     |
| **Deployment** | @AttissNgo <attiss@interoplabs.io>, @milapsheth <milap@interoplabs.io> |

| **Network**  | **Deployment Status** | **Date**    |
| ------------ | --------------------- | ----------- |
| **Stagenet** | Completed             | 2025-09-08  |
| **Testnet**  | Completed             | 2025-09-05 |

- [Releases](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0)

## Background

Celo Sepolia will replace Alfajores testnet when Holesky sunsets in September 2025. This is the ITS v2.2.0 deployment for Celo Sepolia in stagenet and testnet. Mainnet is not affected.

## Deployment

Ensure that [Celo-Sepolia GMP](releases/evm/2025-09-Celo-Sepolia-GMP-v6.0.6.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<stagenet|testnet>
CHAIN=celo-sepolia
```

| Network      | `deployer address`                           |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Stagenet / Testnet

```bash
ts-node evm/deploy-its.js -s "v2.2.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Set Celo-Sepolia as trusted chain on remote ITS contracts

#### Note: Ensure that Celo-Sepolia is registered on ITS hub

Set `Celo-Sepolia` as trusted chain on all EVM chains

```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

Set `Celo-Sepolia` as trusted chain on Sui

```bash
ts-node sui/its.js add-trusted-chains $CHAIN
```

Set `Celo-Sepolia` as trusted chain on Stellar

```bash
ts-node stellar/its.js add-trusted-chains $CHAIN
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n $CHAIN -y
```

- Verify the token manager proxy contract once an ITS token is deployed on `Celo-Sepolia` and then mark it as a proxy.

- EVM Checklist

```bash
# Create a token on `Celo-Sepolia`
ts-node evm/interchainTokenFactory.js --action deployInterchainToken --minter [minter-address] --name "test" --symbol "TST" --decimals 6 --initialSupply 10000 --salt "salt1234" -n $CHAIN

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt1234" --gasValue [gas-value] -y -n $CHAIN

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [token-id] [recipient] 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [token-id] [destination-address] 1 --gasValue [gas-value] -n [destination-chain]
```

- Sui Checklist

```bash
# Deploy Token on Sui
ts-node sui/its-example deploy-token --origin TST "Test Token" 6

# Send Token Deployment to `Celo-Sepolia`
ts-node sui/its-example send-deployment TST $CHAIN [gas-value]

# Send Token to `Celo-Sepolia`
ts-node sui/its-example send-token TST $CHAIN [destination-address] [gas-value] 1

# Send token back to Sui from `Celo-Sepolia`
ts-node evm/its.js --action interchainTransfer --destinationChain sui --tokenId [token-id] --destinationAddress [recipient] --amount 1 --gasValue [gas-value] -n $CHAIN
```

- Stellar Checklist

```bash
# Deploy token to Stellar from `Celo-Sepolia`
ts-node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain stellar-2025-q3 --salt "salt1234" --gasValue [gas-value] -y -n $CHAIN

# Transfer token to Stellar
ts-node evm/its.js interchain-transfer stellar-2025-q3 [token-id] [recipient] 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from Stellar
ts-node stellar/its.js interchain-transfer [token-id] $CHAIN [destination-address] 1 --gas-amount [gas-amount]
```
