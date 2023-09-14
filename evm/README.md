## EVM deployment scripts

This folder contains deployment scripts for the following contracts.

#### Setup

`npm ci`

For contract verification to work, copy over the appropriate build `artifacts` and `contracts` folder from the source repo into this repo. And update the hardhat config to use the same compiler version and optimizer runs setting.

You also need to create a `keys.json` file. See `evm/.example.keys.json` for an example.

### AxelarGateway

1. Compile the contracts from the source repo
2. Copy the `artifacts` folder at the root level of this repo
3. Add the deployer private key in `.env` folder (see `.example.env` for reference)
4. Add additional params in `.env` such as admin addresses, governance etc.
5. If you'd like to auto-verify the contract on the explorer, then add the explorer API key under `keys.json` (see `.example.keys.json`), and add `--verify` flag
6. Run the following depending on the service,
   `node evm/deploy-gateway-v5.0.x.js --env testnet -n fantom`

#### Gateway Upgrade

1. When upgrading the gateway, the proxy contract will be reused.
2. Depending on the upgrade process, Axelar auth and token deployer helper contracts might be reused as well.
3. `node evm/deploy-gateway-v5.0.x.js -e testnet -n fantom --reuseProxy` OR
4. `node evm/deploy-gateway-v5.0.x.js -e testnet -n fantom --reuseProxy --reuseHelpers`
5. This sets the new `implementation` in the chain config.
6. Upgrade to the new implementation contract
   `node evm/deploy-gateway-v5.0.x.js -e testnet -n fantom --upgrade`

### AxelarGasService and AxelarDepositService

1. Compile the contracts from the source repo
2. Copy the `artifacts` folder at the root level of this repo
3. Add the deployer private key in `.env` folder (see `.example.env` for reference)
4. If you'd like to auto-verify the contract on the explorer, then add the explorer API key under `keys.json` (see `.example.keys.json`), and add `--verify` flag
5. Use the `--upgrade` flag to upgrade the contract instead
6. Run the following depending on the service,
   `node evm/deploy-upgradable.js deploy --env testnet -n fantom -c AxelarGasService -a ../artifacts/contracts/gas-service/`

### InterchainTokenService

Install and copy default setting with

```
npm ci && cp example.env .env
```

To test the Interchain Token Service deployment

```
node evm/deploy-its -n ${chain-name} -s [salt]
```

Run again with `-v only` to verify deployed contracts.

You can change `.env` to run the above script to testnet instead of local. Change the `SALT` to get a new address.
