# Robinhood ITS v2.2.0

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

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0)

## Background

- This is the Robinhood ITS release.

## Deployment

Ensure that [Robinhood GMP](../evm/2026-09-Robinhood-GMP.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Set `CHAIN=robinhood` in your shell for the commands below.

```yaml
PRIVATE_KEY=<deployer private key>
ENV=testnet
CHAIN=robinhood
CHAINS=robinhood
```

| Network     | `deployer address`                           |
| ----------- | -------------------------------------------- |
| **Testnet** | `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` |

### Testnet

```bash
ts-node evm/deploy-its.js -s "v2.2.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register Robinhood ITS on ITS Hub

Note: this step should be performed during the [Cosmwasm GMP deployment](../cosmwasm/2026-09-Robinhood-GMP.md). If it was **not**, register Robinhood ITS on ITS Hub now:

```bash
ts-node cosmwasm/contract.ts its-hub-register-chains $CHAIN \
    --governance
```

If contracts are not deployed yet add the following to `contracts` in the `$CHAIN` config within `ENV.json`:

| Network     | `ITS_EDGE_CONTRACT`                          |
| ----------- | -------------------------------------------- |
| **Testnet** | `0x3270540e5d2857E69BEC34c6EcFa02cD813aCb42` |

```json
{
    "InterchainTokenService": {
        "address": "$ITS_EDGE_CONTRACT"
    }
}
```

## Set Robinhood as trusted chain on remote ITS contracts

Set Robinhood as trusted chain on remote ITS contracts for EVM and non-EVM chains.

```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

## Checklist

```bash
# Create a token on Robinhood
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name [name] --symbol [symbol] --decimals [decimals] --initialSupply [initial-supply] --minter [minter] --salt "salt1234" -n $CHAIN

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token [destination-chain] --salt "salt1234" -n $CHAIN

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer --destinationChain [destination-chain] --tokenId [token-id] --destinationAddress [recipient] --amount 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer --destinationChain $CHAIN --tokenId [token-id] --destinationAddress [destination-address] --amount 1 --gasValue [gas-value] -n [destination-chain]
```
