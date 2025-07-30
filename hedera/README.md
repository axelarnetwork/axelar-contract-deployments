# Deploy Hedera ITS contracts

## Setup

Clone [Hedera fork of ITS](http://github.com/commonprefix/interchain-token-service/tree/hedera-its) and checkout the `hedera-its` branch. Make sure the ITS directory is called `interchain-token-service` and lives alongside this repo's directory — otherwise change the path in `package.json` and reinstall dependencies via npm.

Populate the `.env` with `PRIVATE_KEY` and `HEDERA_ID` you can get on [Hedera Portal](http://portal.hedera.com). Additionally set the `HEDERA_NETWORK` variable to `mainnet`/`testnet`/`previewnet`/`local`.

```sh
PRIVATE_KEY = '0x..'
HEDERA_ID = '0.0.xxxxxxx'
HEDERA_NETWORK = 'testnet'
```

## Scripts

### Deploy HTS Library

Deploy the HTS (Hedera Token Service) library contract required for interacting with the Hedera precompiles.

```bash
node hedera/deploy-hts-lib.js [options]
```

**Options:**
- `--gas <amount>` - Gas limit for deployment (default: 3,000,000)
- `--output <file>` - Output file path to save deployment info
- `--accountId <id>` - Hedera account ID (optional)
- `--privateKey <key>` - Private key for the account (optional)
- `--hederaNetwork <network>` - Hedera network (optional)

**Example with default options and .env:**
```bash
node hedera/deploy-hts-lib.js --gas <amount> --output <file>
```

### Associate Token

Associate a token with a Hedera account. This is required before an account can receive or interact with an HTS token.

```bash
node hedera/associate-token.js <tokenId>
```

**Arguments:**
- `<tokenId>` - Token ID in Hedera format (0.0.xxxxx) or EVM address format

**Options:**
- `--accountId <id>` - Hedera account ID (optional)
- `--privateKey <key>` - Private key for the account (optional)
- `--hederaNetwork <network>` - Hedera network (optional)

**Example:**
```bash
node hedera/associate-token.js 0.0.123456
```

### Fund with WHBAR

Fund an address with WHBAR (Wrapped HBAR) by depositing HBAR. WHBAR is the ERC-20 compatible version of HBAR used in smart contracts. See more about WHBAR [here](https://docs.hedera.com/hedera/core-concepts/smart-contracts/wrapped-hbar-whbar).
Addresses of `WHBAR` contracts can be found [here](https://docs.hedera.com/hedera/core-concepts/smart-contracts/wrapped-hbar-whbar#contract-deployments).

```bash
node hedera/fund-whbar.js <receiverAddress> --amount <amount> --whbarAddress <address>
```

**Arguments:**
- `<receiverAddress>` - Address to fund with WHBAR (required)

**Options:**
- `--whbarAddress <address>` - Address of the WHBAR contract (optional, defaults to `WHBAR_ADDRESS` env var)
- `--amount <amount>` - Amount of HBAR to deposit and convert to WHBAR (optional, defaults to WHBAR_AMOUNT env var)
- `--privateKey <key>` - Private key for funding account (optional)
- `--yes` - Skip confirmation prompt

**Example:**
```bash
node hedera/fund-whbar.js --to 0x742d35cc6634c0532925a3b8d098e9c6084b66e6 --whbarAddress 0x... --amount 10
```

### Deploy ITS Contracts

After deploying the HTS library and populating its address in `.env`, deploy the complete ITS contract suite:

```bash
node hedera/deploy-its.js -s "salt123 devnet-amplifier" --proxySalt 'salt123 devnet-amplifier' -m create2 -e devnet-amplifier -n hedera
```

## Common Options

All scripts support these base options:
- `--accountId <id>` - Hedera account ID (can use HEDERA_ID env var)
- `--privateKey <key>` - Private key for the account (can use `PRIVATE_KEY` env var)
- `--hederaNetwork <network>` - Hedera network: testnet or mainnet (can use HEDERA_NETWORK env var)

## Environment Variables

Set these Hedera-specific vars in your `.env` file:
- `PRIVATE_KEY` - Your Hedera private key
- `HEDERA_ID` - Your Hedera account ID
- `HEDERA_NETWORK` - Network to use (testnet/mainnet)
- `HTS_LIB_ADDRESS` - WHBAR contract address in EVM format (0x...)

Otherwise consult the canonical `evm/deploy-its.js` script for more details on the ITS deployment process.
