## Berachain ITS v2.1.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-05-23 |
| **Stagenet**         | Deployed              | 2025-05-28 |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.1.0)

## Background

- This is the Berachain ITS release.

## Deployment

Ensure that [Berachain GMP](../evm/2025-05-Berachain-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Local environment variable `CHAIN` should be set to `berachain`.

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
node evm/deploy-its.js -s "v2.1.0 devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
node evm/deploy-its.js -s "v2.1.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register Berachain ITS on ITS Hub

Please refer to `$DEPOSIT_VALUE` and `$RUN_AS_ACCOUNT` from [Berachain GMP Amplifier](../cosmwasm/2025-04-Berachain-GMP-v6.0.4.md).

```bash
node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" \
```

## Set Berachain as trusted chain on remote ITS contracts

Set Berachain as trusted chain on remote ITS contracts for EVM and non-EVM chains.

```bash
node evm/its.js set-trusted-chains all hub -n berachain
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
node evm/its.js checks -n $CHAIN -y
```

- Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `wallet` below with the deployer key
node evm/interchainTokenFactory.js --action deployInterchainToken --minter [minter-address] --name "test" --symbol "TST" --decimals 6 --initialSupply 10000 --salt "salt1234" -n $CHAIN

# Deploy token to a remote chain
node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt1234" --gasValue 1000000000000000000 -y -n $CHAIN

# Transfer token to remote chain
node evm/its.js interchain-transfer [destination-chain] [tokenId] [recipient] 1 --gasValue 1000000000000000000 -n $CHAIN

# Transfer token back from remote chain
node evm/its.js interchain-transfer $CHAIN [tokenId] [destination-address] 1 --gasValue 1000000000000000000 -n [destination-chain]
```
