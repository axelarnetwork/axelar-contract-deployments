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
node hedera/fund-whbar.js <receiverAddress> --amount <amount> -e <env> -n <chainName>
```

**Arguments:**
- `<receiverAddress>` - Address to fund with WHBAR (required)

**Options:**
- `--chainName <chainName>` - Chain name to get WHBAR address from (can use CHAIN env var)
- `--env <env>` - Environment configuration to use: mainnet, stagenet, testnet, devnet-amplifier (defaults to devnet-amplifier, can use ENV env var)
- `--amount <amount>` - Amount of HBAR to deposit and convert to WHBAR (can use WHBAR_AMOUNT env var)
- `--privateKey <key>` - Private key for funding account (optional)
- `--yes` - Skip confirmation prompt

**Example:**
```bash
node hedera/fund-whbar.js 0x742d35cc6634c0532925a3b8d098e9c6084b66e6 --chainName hedera --env devnet-amplifier --amount 10
```

### Approve WHBAR for InterchainTokenFactory

Approve WHBAR spending for the InterchainTokenFactory contract. This allows the factory to spend your WHBAR tokens when deploying new interchain tokens locally on the same chain.

```bash
node hedera/approve-factory-whbar.js --amount <amount> --chainName <chainName> --env <env>
```

**Options:**
- `--chainName <chainName>` - Chain name to get WHBAR and InterchainTokenFactory addresses from (can use CHAIN env var)
- `--env <env>` - Environment configuration to use: mainnet, stagenet, testnet, devnet-amplifier (defaults to devnet-amplifier, can use ENV env var)
- `--amount <amount>` - Amount to approve (use "max" for maximum uint256, defaults to "max")
- `--privateKey <key>` - Private key for the account (optional)
- `--yes` - Skip confirmation prompt

**Example:**
```bash
node hedera/approve-factory-whbar.js --chainName hedera --env devnet-amplifier --amount max
```

### Associate Token with Account

Associate a Hedera token with your account. This is required before you can receive or interact with HTS (Hedera Token Service) tokens. The script accepts both Hedera token IDs (0.0.xxxxx format) and EVM addresses.

```bash
node hedera/associate-token.js <tokenId>
```

**Arguments:**
- `<tokenId>` - Token ID in Hedera format (0.0.xxxxx) or EVM address (0x...)

**Options:**
- `--accountId <id>` - Hedera account ID (can use HEDERA_ID env var)
- `--privateKey <key>` - Private key for the account (can use PRIVATE_KEY env var)
- `--hederaNetwork <network>` - Hedera network: testnet or mainnet (can use HEDERA_NETWORK env var)
- `--yes` - Skip confirmation prompt

**Example:**
```bash
node hedera/associate-token.js 0.0.123456
```

**Note:** If the token is already associated with your account, the script will detect this and exit successfully without performing a duplicate association.

### Token Creation Price Management

Manage token creation pricing for the InterchainTokenService. Query prices in tinycents/tinybars and set prices in tinycents.

#### Query Token Creation Price

Get the current token creation price in all formats: tinycents, cents, USD, and optionally tinybars/HBAR.

```bash
node hedera/token-creation-price.js price --chainName <chainName> --env <env>
```

**Options:**
- `--chainName <chainName>` - Chain name to get InterchainTokenService address from (can use CHAIN env var)
- `--env <env>` - Environment configuration to use: mainnet, stagenet, testnet, devnet-amplifier (defaults to devnet-amplifier, can use ENV env var)

**Example:**
```bash
node hedera/token-creation-price.js price --chainName hedera --env devnet-amplifier
```

#### Set Token Creation Price

Set the token creation price in tinycents.

```bash
node hedera/token-creation-price.js set-price <price> --chainName <chainName> --env <env>
```

**Arguments:**
- `<price>` - Price value in tinycents (1 USD = 100 cents = 10,000,000,000 tinycents)

**Options:**
- `--chainName <chainName>` - Chain name to get InterchainTokenService address from (can use CHAIN env var)
- `--env <env>` - Environment configuration to use: mainnet, stagenet, testnet, devnet-amplifier (defaults to devnet-amplifier, can use ENV env var)
- `--privateKey <key>` - Private key for the account (optional)
- `--yes` - Skip confirmation prompt

**Examples:**
```bash
# Set price to 2 USD (2 * 100 * 100,000,000 = 20,000,000,000 tinycents)
node hedera/token-creation-price.js set-price 20000000000 --chainName hedera --env devnet-amplifier

# Set price to 50 cents (50 * 100,000,000 = 5,000,000,000 tinycents)
node hedera/token-creation-price.js set-price 5000000000 --chainName hedera --env devnet-amplifier
```

**Note:** The script will display the equivalent values in cents and USD for confirmation before setting the price.
