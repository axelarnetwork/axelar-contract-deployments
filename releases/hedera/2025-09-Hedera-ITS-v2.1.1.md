# Hedera ITS v2.1.1

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @rista404 <ristic@commonprefix.com> |
| **Deployment** | @rista404 <ristic@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**    |
| -------------------- | --------------------- | ----------- |
| **Devnet Amplifier** | NA                    | NA          |
| **Stagenet**         | Deployed              | 2025-10-07  |
| **Testnet**          | -                     | TBD         |
| **Mainnet**          | -                     | TBD         |

## Background

Upgrade of the Hedera-fork of Interchain Token Service. Contracts impacted: `TokenManager` (implementation).

Changes in the release:

1. Lower the approval amount to the max supply of a token with finite supply. This prevents issues when registering tokens with finite max supply. [See commit.](https://github.com/commonprefix/interchain-token-service/commit/c6fda1781dfb0a00d9e74e420cca7beba9bbcda8) <!-- skip-check -->

## Deployment

Ensure that [Hedera ITS](./2025-07-Hedera-ITS-v2.1.0) is deployed first.

Follow `hedera/README.md` for Hedera account setup and in-depth `.env` configuration.

Make sure to checkout [c6fda1781dfb0a00d9e74e420cca7beba9bbcda8](https://github.com/commonprefix/interchain-token-service/commit/c6fda1781dfb0a00d9e74e420cca7beba9bbcda8), run `npx hardhat compile` in `interchain-token-service`, and run `npm i` in this repo after changing the `package.json`. <!-- skip-check -->

Create an `.env` config

```sh
PRIVATE_KEY=<deployer hex private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAINS=hedera
# + hedera specific env vars, see hedera/README.md
```

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Devnet Amplifier

```bash
# Deploy new implementation
ts-node evm/deploy-its.js -s "v2.1.1 devnet-amplifier" -m create2 --reuseProxy

ts-node evm/deploy-its.js --upgrade
```

### Stagenet / Testnet / Mainnet

```bash
# Deploy new implementation
ts-node evm/deploy-its.js -s "v2.1.1" -m create2 --reuseProxy

ts-node evm/deploy-its.js --upgrade
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n $CHAIN -y
```

- Verify the token manager proxy contract once an ITS token is deployed on `<ChainName>` and then mark it as a proxy.

> Note: before transferring any tokens to an account on Hedera, that account must be associated with the token. Use the `associate-token.js` script to associate the token with the account, see `hedera/README.md` for more details. Use `gas-value` in in tinybars (8 decimals).

- EVM Checklist

```bash
# Fund user with some WHBAR
ts-node hedera/fund-whbar.js [user-address] --amount 100 -n $CHAIN

# Approve factory to spend WHBAR
ts-node hedera/approve-factory-whbar.js -n $CHAIN

# Create a token on Hedera
ts-node evm/interchainTokenFactory.js --action deployInterchainToken --minter [minter-address] --name "test" --symbol "TST" --decimals 6 --salt "salt1234" --initialSupply 0 -n $CHAIN --gasValue [gas-value]

# Record the newly created token id and address from the output.

# Associate with the token address
ts-node hedera/associate-token.js [token-address]

# Mint some tokens via the TokenManager
ts-node evm/its.js mint-token [token-id] [to] [amount]

# Record Token Manger Address

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt1234" --gasValue [gas-value] -y

# Approve token manager to spend tokens
ts-node evm/its.js approve [token-id] [spender] [amount] 

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [token-id] [recipient] 1 --gasValue [gas-value]

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [token-id] [destination-address] 1 --gasValue [gas-value] -n [destination-chain]
```

Additionally, to test the patch itself:

```bash
node evm/interchainTokenFactory.js --action registerCanonicalInterchainToken -n $CHAIN --tokenAddress [token-address]
```

To find an appropriate token address, go to [HashScan](https://hashscan.io/), either testnet or mainnet, and look for a fungible token created with the `max supply` field set to a finite value (e.g. `1000000`).
