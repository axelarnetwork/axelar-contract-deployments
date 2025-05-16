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
node evm/deploy-its.js -s "v2.1.0 devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
node evm/deploy-its.js -s "v2.1.0" -m create2 --proxySalt 'v1.0.0'
```

### Verify Upgraded ITS Contracts

Please follow this [instruction](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/evm#contract-verification) to verify ITS contracts on EVM chains.

## Register &lt;ChainName&gt; ITS on ITS Hub

```bash
node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub" 
```

## Set &lt;ChainName&gt; as trusted chain on remote ITS contracts

Set `<ChainName>` as trusted chain on remote ITS contracts for EVM and non-EVM chains.

```bash
node evm/its.js set-trusted-chains $CHAIN hub -n all
```

## Checklist

The following checks should be performed after the rollout.

- Run post-deployment checks.

```bash
node evm/its.js checks -n $CHAIN -y
```

- Run the following for two EVM chains (one Amplifier, one consensus, with different decimals for each token)

```bash
# Create a token on chain. Substitute the `minter-address` below with the deployer key
node evm/interchainTokenFactory.js --action deployInterchainToken --minter [minter-address] --name "test" --symbol "TST" --decimals 6 --initialSupply 10000 --salt "salt1234" -n $CHAIN

# Deploy token to a remote chain
 node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt1234" --gasValue 1000000000000000000 -y -n $CHAIN

# Transfer token to remote chain
node evm/its.js interchain-transfer [destination-chain] [tokenId] [recipient] 1 --gasValue 1000000000000000000 -n $CHAIN

# Transfer token back from remote chain
node evm/its.js interchain-transfer $CHAIN [tokenId] [destination-address] 1 --gasValue 1000000000000000000 -n [destination-chain]
```
