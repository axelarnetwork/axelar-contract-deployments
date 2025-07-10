# &lt; ChainName &gt; GMP vX.X.X

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @yourGithubUsername <user@interoplabs.io> |
| **Deployment** | @yourGithubUsername <user@interoplabs.io> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

- [Releases] add link to Github release here

## Background

Describe release content here

## Deployment

Ensure that [<Chain's GMP>](../evm/path-to-GMP-release-doc) is deployed first. 

```bash
# Clone latest main and update deps
npm ci
```

Create an `.env` config

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAIN=<chain name>
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

## Set &lt;ChainName&gt; as trusted chain on remote ITS contracts

#### Note: Ensure that &lt;ChainName&gt; is registered on ITS hub

Set `<ChainName>` as trusted chain on all EVM chains
```bash
ts-node evm/its.js set-trusted-chains $CHAIN hub -n all
```

Set `<ChainName>` as trusted chain on Sui

```bash
ts-node sui/its.js add-trusted-chains $CHAIN
```

Set `<ChainName>` as trusted chain on Stellar

```bash
ts-node stellar/its.js add-trusted-chains $CHAIN
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
ts-node evm/its.js checks -n $CHAIN -y
```

- Verify the token manager proxy contract once an ITS token is deployed on `<ChainName>` and then mark it as a proxy.

- EVM Checklist

```bash
# Create a token on `<ChainName>`
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
# Deploy Token on sui
ts-node sui/its-example deploy-token --origin TST "Test Token" 6

# Send Token Deployment to `<ChainName>`
ts-node sui/its-example send-deployment TST $CHAIN [gas-value]

# Send Token to `<ChainName>`
ts-node sui/its-example send-token TST $CHAIN [destination-address] [gas-value] 1

# Send token back to sui from `<ChainName>`
ts-node evm/its.js --action interchainTransfer --destinationChain sui --tokenId [token-id] --destinationAddress [recipient] --amount 1 --gasValue [gas-value] -n $CHAIN
```

- Stellar Checklist

```bash
# Deploy token to a stellar from `<ChainName>`
ts-node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain stellar --salt "salt1234" --gasValue [gas-value] -y -n $CHAIN

# Transfer token to stellar
ts-node evm/its.js interchain-transfer stellar [token-id] [recipient] 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from stellar
ts-node stellar/its.js interchain-transfer [token-id] $CHAIN [destination-address] 1 --gas-amount [gas-amount]
```
