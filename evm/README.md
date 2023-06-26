## EVM deployment scripts

This folder contains deployment scripts for the following contracts.

#### Setup

`npm ci`

### AxelarGateway

TODO

### AxelarGasService and AxelarDepositService

1. Compile the contracts from the source repo
2. Copy the `artifacts` folder at the root level of this repo
3. Add the deployer private key in `.env` folder (see `.example.env` for reference)
4. If you'd like to auto-verify the contract on the explorer, then add the explorer API key under `info/keys.json` (see `.example.keys.json`), and add `--verify` flag
5. Use the `--upgrade` flag to upgrade the contract instead
6. Run the following depending on the service,
`node evm/deploy-upgradable.js deploy --env testnet -n fantom -c AxelarGasService -a ../artifacts/contracts/gas-service/`
