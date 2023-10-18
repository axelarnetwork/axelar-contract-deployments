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

### Contract Verification

#### Prerequisites

- Command for verification will eventually be run from the directory specified with the command using `--dir` which contains the contract artifacts directory ex: `/home/axelar/axelar-cgp-solidity`. So you will need to provide a path for the directory, which contains the `artifacts` directory.

- Checkout to the version of contracts to verify in the directory provided to the command before compiling artifacts used by the command. (v4.3 is the latest version of contracts as of now)

```bash
git checkout vX.Y.Z
- Copy the artifacts generated in `axelar-cgp-solidity` to this repo as the verify script will need it.

- The directory should also contain a `keys.json` for info on the chain and explorer. The expected location of the file can be verified in `hardhat.config.js`. You need to create a `keys.json` in the following format in the expected location if it doesn't exist:

   ```bash
   {
      "chains": {
         "fantom": {"api": "API_KEY_FANTOM"}, 
         "ethereum": {"api": "API_KEY_ETH"}, 
         "mantle": {"api": "ETHERSCAN_API_KEY_MANTLE"}},
      "accounts": ["PRIVATE_KEY_1", "PRIVATE_KEY_2"]
   }
   ```
#### Help

To get details of options provided in the command run:
```bash
node evm/verify-contract.js --help

#### Example

Here is an example command to run the `evm/verify-contract.js` script:


- The following command will verify Axelar wrapped tokens deployed via the gateway (`BurnableMintableCappedERC20` contract) on the chain in appropriate environment. The address will be retrieved from the gateway by default but can be provided explicitly.

```bash
node evm/verify-contract.js -e mainnet -n [chain] -c BurnableMintableCappedERC20 --dir /path/to/axelar-cgp-solidity --args axlUSDC
