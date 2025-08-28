# Hedera ITS v2.1.0

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @rista404 <ristic@commonprefix.com> |
| **Deployment** | @rista404 <ristic@commonprefix.com> |

| **Network**          | **Deployment Status** | **Date**    |
| -------------------- | --------------------- | ----------- |
| **Devnet Amplifier** | Deployed              | 2025-08-21  |
| **Stagenet**         | Deployed              | 2025-07-30  |
| **Testnet**          | -                     | TBD         |
| **Mainnet**          | -                     | TBD         |

- [Release](https://github.com/commonprefix/interchain-token-service/tree/01ac9020896b6e16577a9d922f6b7e23baae9145)

## Background

Deployment of the Hedera-fork of Interchain Token Service.

## Deployment

Ensure that [Hedera GMP](../evm/2025-04-Hedera-GMP-v6.0.4.md) is deployed first.

Follow `hedera/README.md` for Hedera account setup and in-depth `.env` configuration.

Create an `.env` config

```sh
PRIVATE_KEY=<deployer hex private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAIN=<chain name>
# + hedera specific env vars, see hedera/README.md
```

| Network              | `deployer address`                           |
| -------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

### Deploy HTS Library

```bash
ts-node hedera/deploy-hts-lib.js
```

After this step, update the `.env` file with the `HTS_LIB_ADDRESS` variable set to the deployed HTS library address in EVM format (0x...). Alternatively you can pass the address as an argument to the deploy script, via `--htsLibraryAddress <address>`.

### Check WHBAR addresses

See the [Hedera docs](https://docs.hedera.com/hedera/core-concepts/smart-contracts/wrapped-hbar-whbar#contract-deployments) for the WHBAR contract addresses. Make sure to use the correct address for your network.

You can set the `WHBAR_ADDRESS` environment variable in your `.env` file, or pass it as an argument to the deploy script, via `--whbarAddress <address>`.

### Devnet Amplifier

```bash
ts-node hedera/deploy-its.js -s "v2.1.0 devnet-amplifier" -m create2 --proxySalt 'v1.0.0 devnet-amplifier'
```

### Stagenet / Testnet / Mainnet

```bash
ts-node hedera/deploy-its.js -s "v2.1.0" -m create2 --proxySalt 'v1.0.0'
```

### Register ITS edge contract on ITS Hub

##### Setup truncation params in InterchainTokenService contract

As HTS tokens use uint64, set that as the max uint and 6 decimals. Add following under `config.axelar.contracts.InterchainTokenService`:

```
"$CHAIN": {
    "maxUintBits": 64,
    "maxDecimalsWhenTruncating": 6
},
```
#### Register ITS edge contract on ITS Hub

Before proceeding, confirm ITS contract is deployed and is mentioned in `ENV.json`. Run:

```bash
ts-node cosmwasm/submit-proposal.js \
    its-hub-register-chains $CHAIN \
    -t "Register $CHAIN on ITS Hub" \
    -d "Register $CHAIN on ITS Hub"
```

### Fund ITS with WHBAR

```bash
ts-node hedera/fund-whbar.js <its_address> --whbarAddress <whbar_address> --amount 10
```

The `--amount` is a value in HBAR. You can optionally skip the `--whbarAddress` argument if you have set the `WHBAR_ADDRESS` environment variable in your `.env` file.

### Fund user with WHBAR and approve factory

For local factory deployments, the user needs to have some WHBAR to pay for token creation. Repeat the step above with the user's address, afterwhich sufficent allowance needs to be given to the factory contract.

### Verify Upgraded ITS Contracts

> TODO: Needs instructions for Hedera, missing in GMP Release.

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

> Note: before transfering any tokens to an account on Hedera, that account must be associated with the token. Use the `associate-token.js` script to associate the token with the account, see `hedera/README.md` for more details.

- EVM Checklist

```bash
# Fund user with some WHBAR
ts-node hedera/fund-whbar.js [user-address] --whbarAddress [whbar-address] --amount 100

# Approve factory to spend WHBAR
ts-node hedera/approve-factory-whbar.js --whbarAddress [whbar-address] -n $CHAIN

# Create a token on Hedera
ts-node evm/interchainTokenFactory.js --action deployInterchainToken --minter [minter-address] --name "test" --symbol "TST" --decimals 6 --salt "salt1234" --initialSupply 0 -n $CHAIN

# Record the newly created token address from the output.

# Associate with the token address
ts-node hedera/associate-token.js [token-address]

# Mint some tokens via the TokenManager

# Deploy token to a remote chain
ts-node evm/interchainTokenFactory.js --action deployRemoteInterchainToken --destinationChain [destination-chain] --salt "salt1234" --gasValue [gas-value] -y -n $CHAIN

# Approve token manager to spend tokens

# Transfer token to remote chain
ts-node evm/its.js interchain-transfer [destination-chain] [token-id] [recipient] 1 --gasValue [gas-value] -n $CHAIN

# Transfer token back from remote chain
ts-node evm/its.js interchain-transfer $CHAIN [token-id] [destination-address] 1 --gasValue [gas-value] -n [destination-chain]
```
