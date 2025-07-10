## Plume ITS v2.1.0

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @AttissNgo <attiss@interoplabs.io> |
| **Deployment** | @AttissNgo <attiss@interoplabs.io>, @milapsheth <milap@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-04-30 |
| **Stagenet**         | Completed             | 2025-05-07 |
| **Testnet**          | Completed             | 2025-05-27 |
| **Mainnet**          | Completed             | 2025-06-11 |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.1.0)

## Background

- This is the Plume ITS release.

## Deployment

Ensure that [Plume GMP](../evm/2025-05-Plume-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Local environment variable `CHAIN` should be set to `plume`.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=xyz
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

## Register Plume ITS on ITS Hub

Please refer to `$DEPOSIT_VALUE` and `$RUN_AS_ACCOUNT` from [Plume GMP Amplifier](../cosmwasm/2025-05-Plume-GMP-v6.0.4.md).

```bash
ts-node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" \
    --deposit $DEPOSIT_VALUE \
    --runAs $RUN_AS_ACCOUNT
```

## Set Plume as trusted chain on remote ITS contracts

Set Plume as trusted chain on remote ITS contracts for EVM and non-EVM chains.

```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n $CHAIN -y
```

- Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `minter-address` below with the deployer key
ts-node evm/interchainTokenFactory.js --action deployInterchainToken --minter [minter-address] --name "test" --symbol "TST" --decimals 6 --initialSupply 10000 --salt "salt1234" -n $CHAIN

# Deploy token to a remote chain
 ts-node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt1234" --gasValue 1000000000000000000 -y -n $CHAIN

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [tokenId] [recipient] 1 --gasValue 1000000000000000000 -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [tokenId] [destination-address] 1 --gasValue 1000000000000000000 -n [destination-chain]
```
