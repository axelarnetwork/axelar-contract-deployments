# EVM deployment scripts

This folder contains deployment scripts for the following contracts.

## Setup

`npm ci`

For contract verification to work, copy over the appropriate build `artifacts` and `contracts` folder from the source repo into this repo. And update the hardhat config to use the same compiler version and optimizer runs setting.

You also need to create a `keys.json` file. See `evm/.example.keys.json` for an example.

## AxelarGateway

1. Compile the contracts from the source repo
2. Copy the `artifacts` folder at the root level of this repo
3. Add the deployer private key in `.env` folder (see `.example.env` for reference)
4. Add additional params in `.env` such as admin addresses, governance etc.
5. If you'd like to auto-verify the contract on the explorer, then add the explorer API key under `keys.json` (see `.example.keys.json`), and add `--verify` flag
6. Run the following depending on the service,
   `node evm/deploy-gateway-v5.0.x.js --env testnet -n fantom`

## Gateway Upgrade

1. When upgrading the gateway, the proxy contract will be reused.
2. Depending on the upgrade process, Axelar auth and token deployer helper contracts might be reused as well.
3. `node evm/deploy-gateway-v5.0.x.js -e testnet -n fantom --reuseProxy` OR
4. `node evm/deploy-gateway-v5.0.x.js -e testnet -n fantom --reuseProxy --reuseHelpers`
5. This sets the new `implementation` in the chain config.
6. Upgrade to the new implementation contract
   `node evm/deploy-gateway-v5.0.x.js -e testnet -n fantom --upgrade`

## AxelarGasService and AxelarDepositService

1. Compile the contracts from the source repo
2. Copy the `artifacts` folder at the root level of this repo
3. Add the deployer private key in `.env` folder (see `.example.env` for reference)
4. If you'd like to auto-verify the contract on the explorer, then add the explorer API key under `keys.json` (see `.example.keys.json`), and add `--verify` flag
5. Use the `--upgrade` flag to upgrade the contract instead
6. Run the following depending on the service,
   `node evm/deploy-upgradable.js deploy --env testnet -n fantom -c AxelarGasService -a ../artifacts/contracts/gas-service/`

## InterchainTokenService

Install and copy default setting with

```bash
npm ci && cp example.env .env
```

To test the Interchain Token Service deployment

```bash
node evm/deploy-its -n ${chain-name} -s [salt]
```

Run again with `-v only` to verify deployed contracts.

You can change `.env` to run the above script to testnet instead of local. Change the `SALT` to get a new address.

## Contract Verification

### Prerequisites

- Clone the repo containing the contract source code.

```bash
git clone https://github.com/axelarnetwork/axelar-cgp-solidity.git
```

- Checkout to the version of contracts to verify in the directory provided to the command before compiling artifacts used by the command.

```bash
git checkout vX.Y.Z

npm ci

npm run build
```

- Update `.hardhat.config.js` to have `chains` and `keys` to point to the current repo.

```javascript
const chains = require(`../axelar-contract-deployments/axelar-chains-config/info/${env}.json`);
const keys = readJSON(`../axelar-contract-deployments/keys.json`);
```

- `keys.json` is expected to be in the format described [here](./.example.keys.json).
You can generate the explorer API key via creating an account on the explorer.

### Example

Verify the Axelar gateway contract. `-a [address]` can be optionally specified to override the contract address to verify.

```bash
node evm/verify-contract.js -e mainnet -n [chain] -c AxelarGateway --dir /path/to/axelar-cgp-solidity
```

Verify Axelar wrapped tokens deployed via the gateway (`BurnableMintableCappedERC20` contract) on the chain in appropriate environment. The address will be retrieved from the gateway by default but can be provided explicitly.

```bash
node evm/verify-contract.js -e mainnet -n [chain] -c BurnableMintableCappedERC20 --dir /path/to/axelar-cgp-solidity --args axlUSDC
```

#### Help

To get details of options provided in the command run:

```bash
node evm/verify-contract.js --help
```
