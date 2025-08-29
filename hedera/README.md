# Deploy Hedera ITS contracts

## Setup

Clone [Hedera fork of ITS](http://github.com/commonprefix/interchain-token-service/tree/hedera-its) and checkout the `hedera-its` branch. Set the dependency in `package.json` like so: `"@axelar-network/interchain-token-service": "file:../interchain-token-service"`. Make sure the ITS directory is called `interchain-token-service` and lives alongside this repo's directory — otherwise change the path in `package.json`. Reinstall dependencies via npm and generate artifacts as well.

Populate the `.env` with `PRIVATE_KEY` and `HEDERA_ID` you can get on [Hedera Portal](http://portal.hedera.com). Additionally set the `HEDERA_NETWORK` variable to `mainnet`/`testnet`/`previewnet`/`local`.

```sh
PRIVATE_KEY = '0x..'
HEDERA_ID = '0.0.xxxxxxx'
HEDERA_NETWORK = 'testnet'
```

## Deployment

### Deploy HTS Library

Before deploying the ITS contracts, we must first deploy the HTS (Hedera Token Service) library required for interacting with the Hedera precompiles.

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

After this step, update the chain config in `axelar-chains-config/info/<env>.json` like so:

```json
{
	//...
	"hedera": {
		//...
		"htsLibraryAddress": "0x...", // address of the deployed HTS library
		//...
	}
}
```

### Set WHBAR Address

Inside the same chain config file, set the `whbarAddress` field to the address of the WHBAR contract for your network. See the [Hedera docs](https://docs.hedera.com/hedera/core-concepts/smart-contracts/wrapped-hbar-whbar#contract-deployments) for the WHBAR contract addresses.

```json
{
	//...
	"hedera": {
		//...
		"whbarAddress": "0x...", // address of the WHBAR contract
		//...
	}
}
```

### Deploy ITS Contracts

Finally deploy the complete ITS contract suite:

```bash
node evm/deploy-its.js -s "salt123 devnet-amplifier" --proxySalt 'salt123 devnet-amplifier' -m create2 -e devnet-amplifier -n hedera
```

## Utility Scripts

### Common Options

All scripts support these base options:
- `--accountId <id>` - Hedera account ID (can use HEDERA_ID env var)
- `--privateKey <key>` - Private key for the account (can use `PRIVATE_KEY` env var)
- `--hederaNetwork <network>` - Hedera network: testnet or mainnet (can use HEDERA_NETWORK env var)

### Environment Variables

Set these Hedera-specific vars in your `.env` file:
- `PRIVATE_KEY` - Your Hedera private key
- `HEDERA_ID` - Your Hedera account ID
- `HEDERA_NETWORK` - Network to use (testnet/mainnet)

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
