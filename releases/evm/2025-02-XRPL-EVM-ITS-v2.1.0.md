## XRPL EVM Sidechain ITS v2.1.0

|                | **Owner**                                                                   |
| -------------- | --------------------------------------------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io>                                      |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @talalashraf <talal@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | -                     | TBD        |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | `xrp-evm-test-1`      | 19-02-2025 |
| **Mainnet**          | -                     | TBD        |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v)

## Background

- This is the XRPL EVM sidechain ITS release.

## Deployment

Ensure that [XRPL EVM GMP](../evm/2025-02-XRPL-EVM-GMP-v6.0.4.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Use `all` for `CHAINS` to run the cmd for every EVM chain, or set a specific chain. `CHAIN` should be set to `xrpl-evm` for mainnet, and `xrpl-evm-test-1` for all other networks.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=all
```

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Devnet Amplifier

Amplifier ITS

```bash
# Deploy new implementation
node evm/deploy-its.js -s "v2.1.0 devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
# Deploy new implementation
node evm/deploy-its.js -s "v2.1.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register xrpl-evm ITS on ITS Hub

Please refer to `$DEPOSIT_VALUE` and `$RUN_AS_ACCOUNT` from [XRPL EVM GMP Amplifier](../cosmwasm/2025-02-XRPL-EVM-GMP-v1.0.0.md).

```bash
node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" \
    --deposit $DEPOSIT_VALUE \
    --runAs $RUN_AS_ACCOUNT
```

## Setting up trusted chains on xrplevm

```bash
# Add all trusted chains to xrplevm ITS
node evm/its.js -n $CHAIN --action setTrustedAddress --trustedChain all --trustedAddress hub
```

## Set xrplevm as trusted chain on EVM ITS. Similarly, set xrplevm as a trusted chain for every other non EVM ITS contract

```bash
# Change `PRIVATE_KEY and `ENV` in `.env` from xrplevm to EVM
node evm/its.js -n all --action setTrustedAddress --trustedChain $CHAIN --trustedAddress hub
```

## Setting up trusted chains on xrplevm

```bash
# Register token metadata
node evm/its.js --action registerTokenMetadata --tokenAddress 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE

# Fetch token manager address
node evm/its.js --action tokenManagerAddress --tokenId [tokenId]

# tranfer mintership to token manager
node evm/its.js --action transferMintership --tokenAddress 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE --minter [tokenManager]
```

## Checklist

The following checks should be performed after the rollout

- [ ] Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `wallet` below with the deployer key
node evm/interchainTokenFactory.js --action deployInterchainToken --minter [wallet] --name "test" --symbol "TST" --decimals [decimals] --initialSupply 10000 --salt "salt12345"

# Deploy token to a remote chain
node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination chain] --salt "salt12345" -y

#Transfer token
node evm/its.js --action interchainTransfer --destinationChain [destination chain] --tokenId [tokenId] --destinationAddress [recipient] --amount 1 --gasValue 0

# Ensure GMP call is executed on destination chain, where required
```
