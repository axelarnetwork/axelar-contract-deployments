# # Ripple EVM Sidechain ITS v2.1.0

|                | **Owner**                                                                   |
| -------------- | --------------------------------------------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io>                                      |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io>, @talalashraf <talal@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v)

## Background

- This is the Ripple EVM sidechain ITS release.

## Deployment

Ensure that [Ripple EVM GMP](../evm/2025-02-xrplevm-GMP-v6.4.0.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Use `all` for `CHAINS` to run the cmd for every EVM chain, or set a specific chain.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=all
```

### Devnet Amplifier

Amplifier ITS

```bash
# Deploy new implementation
node evm/deploy-its.js -s "v1.1.0 devnet-amplifier" -m create2
```

### Stagenet / Testnet / Mainnet

```bash
# Deploy new implementation
node evm/deploy-its.js -s "v2.1.0" -m create2
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains. Please note that `testnet` contracts must be verified before `stagenet` and `devnet-amplifier`.

## Register xrplevm ITS on ITS Hub

Please refer to `$DEPOSIT_VALUE` and `$RUN_AS_ACCOUNT` from [Ripple EVM GMP Amplifier](../cosmwasm/2025-02-xrplevm-GMP-v1.0.0.md).

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
node evm/its.js -n xrplevm --action setTrustedAddress --trustedChain all --trustedAddress hub
```

## Set xrplevm as trusted chain on EVM ITS. Similarly, set xrplevm as a trusted chain for every other non EVM ITS contract

```bash
# Change `PRIVATE_KEY and `ENV` in `.env` from xrplevm to EVM
node evm/its.js -n all --action setTrustedAddress --trustedChain xrplevm --trustedAddress hub
```

## Checklist

The following checks should be performed after the rollout

- [ ] Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on each chain. Substitute the `wallet` below with the deployer key
node evm/interchainTokenFactory.js --action deployInterchainToken --minter [wallet] --name "test" --symbol "TST" --decimals [decimals] --initialSupply 10000 --salt "salt12345"

# Register token metadata. Ensure GMP call is executed
node evm/its.js --action registerTokenMetadata --tokenAddress [tokenAddress]
```

- [ ] Run from one chain to link to the remote token

```bash
# Register source token. Record tokenId from output for next steps.
node evm/interchainTokenFactory.js --action registerCustomToken --tokenAddress [tokenAddress] --tokenManagerType 4 --operator [wallet] --salt "salt6789"

# Link to remote token. Ensure GMP call is executed
node evm/interchainTokenFactory.js --action linkToken --destinationChain chain2 --destinationTokenAddress [remote token address] --tokenManagerType 4 --linkParams "0x" --salt "salt6789"
```

- [ ] Fetch tokenManager address for deployed token on both chains

```bash
# Record tokenManager address from output for transferMintership
node evm/its.js --action tokenManagerAddress --tokenId [tokenId]
```

- [ ] Run on both chains

```bash
# Transfer mintership for each token to the token manager
node evm/its.js --action transferMintership --tokenAddress [tokenAddress] --minter [tokenManager]
```

- [ ] Interchain Transfer (both ways)

```bash
node evm/its.js --action interchainTransfer --destinationChain chain2 --tokenId [tokenId] --destinationAddress [recipient] --amount 1 --gasValue 0
```
