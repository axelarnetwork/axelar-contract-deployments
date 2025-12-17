## Monad ITS v2.2.0

|                | **Owner**                       |
| -------------- | ------------------------------- |
| **Created By** | @isi8787 <isaac@interoplabs.io> |
| **Deployment** | @isi8787 <isaac@interoplabs.io> |

| **Network**  | **Deployment Status** | **Date**   |
| ------------ | --------------------- | ---------- |
| **Stagenet** | Complete              | 2025-11-06 |
| **Testnet**  | Completed             | 2025-11-16 |
| **Mainnet**  | Completed             | 2025-11-16 |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v2.2.0)

## Background

- This is the Monad ITS release.

## Deployment

Ensure that [Monad GMP](../evm/2025-05-Monad-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Local environment variable `CHAIN` should be set to `monad`.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=xyz
```

| Network      | `deployer address`                           |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Stagenet / Testnet / Mainnet

```bash
ts-node evm/deploy-its.js -s "v2.2.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register Monad ITS on ITS Hub

```bash
ts-node cosmwasm/contract.ts its-hub-register-chains $CHAIN \
    --governance # omit on devnet-amplifier
```

If contracts are not deployed yet add the following to `contracts` in the `$CHAIN`config within`ENV.json`:

| Network      | `ITS_EDGE_CONTRACT`                          |
| ------------ | -------------------------------------------- |
| **Stagenet** | `0x0FCb262571be50815627C16Eca1f5F3D342FF5a5` |
| **Testnet**  | `0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C` |
| **Mainnet**  | `0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C` |

```json
{
    "InterchainTokenService": {
        "address": "$ITS_EDGE_CONTRACT"
    }
}
```

## Set Monad as trusted chain on remote ITS contracts

Set Monad as trusted chain on remote ITS contracts for EVM and non-EVM chains.

```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

## Checklist

```bash
# Create a token on `<ChainName>`
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name [name] --symbol [symbol] --decimals [decimals] --initialSupply [initial-supply] --minter [minter] --salt "salt1234" -n $CHAIN

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token --destinationChain [destination-chain] --salt "salt1234" -n $CHAIN

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [token-id] [recipient] 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [token-id] [destination-address] 1 --gasValue [gas-value] -n [destination-chain]
```
