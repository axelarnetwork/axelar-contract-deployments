## Hedera ITS v2.1.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** | @blockchainguyy <ayush@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

[Release](https://github.com/axelarnetwork/interchain-token-service/releases/tag/v)

## Background

- This is the Hedera ITS release.

## Deployment

Ensure that [Hedera GMP](../evm/2025-04-Hedera-GMP-v6.0.6.md) is deployed first.

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config. Use `all` for `CHAINS` to run the cmd for every EVM chain, or set a specific chain. `CHAIN` should be set to `hedera` for all networks.

```yaml
PRIVATE_KEY=xyz
ENV=xyz
CHAINS=all
```

| Network              | `deployer address` |
| -------------------- | ------------------ |
| **Devnet-amplifier** | ?                  |
| **Stagenet**         | ?                  |
| **Testnet**          | ?                  |
| **Mainnet**          | ?                  |

### Devnet Amplifier

Amplifier ITS

```bash
# Deploy new implementation
node evm/deploy-its.js -s "ITS v2.1.0 devnet-amplifier" -m create2 --proxySalt 'ITS v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
# Deploy new implementation
node evm/deploy-its.js -s "ITS v2.1.0" -m create2 --proxySalt 'ITS v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register Hedera ITS on ITS Hub

Please refer to `$DEPOSIT_VALUE` and `$RUN_AS_ACCOUNT` from [Hedera GMP Amplifier](../cosmwasm/2025-04-Hedera-GMP-v6.0.6.md).

```bash
node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" \
    --deposit $DEPOSIT_VALUE \
    --runAs $RUN_AS_ACCOUNT
```

## Setting up trusted chains on hedera

```bash
# Add all trusted chains to hedera ITS
node evm/its.js -n $CHAIN --action setTrustedAddress --trustedChain all --trustedAddress hub
```

## Set hedera as trusted chain on EVM ITS. Similarly, set hedera as a trusted chain for every other non EVM ITS contract

```bash
# Change `PRIVATE_KEY and `ENV` in `.env` from hedera to EVM
node evm/its.js -n all --action setTrustedAddress --trustedChain $CHAIN --trustedAddress hub
```

## Checklist

The following checks should be performed after the rollout

- [ ] Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `wallet` below with the deployer key
node evm/interchainTokenFactory.js --action deployInterchainToken --minter [wallet] --name "test" --symbol "TST" --decimals [decimals] --initialSupply 10000 --salt "salt12345"

# Deploy token to a remote chain
node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt12345" -y

#Transfer token
node evm/its.js --action interchainTransfer --destinationChain [destination-chain] --tokenId [tokenId] --destinationAddress [recipient] --amount 1 --gasValue 0

# Ensure GMP call is executed on destination chain, where required
```