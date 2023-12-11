# EVM deployment scripts

This folder contains deployment and operational scripts for various contracts.
By default the version of contracts specified in `package.json` will be used for deployment.

## Setup

`npm ci`

Add the deployer private key in `.env` folder (see `.example.env` for reference).

## AxelarGateway

Deploy the gateway contract.

`node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum`

## Gateway Upgrade

1. When upgrading the gateway, the proxy contract will be reused.
2. Depending on the upgrade process, Axelar auth and token deployer helper contracts might be reused as well.
3. `node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum --reuseProxy` OR
4. `node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum --reuseProxy --reuseHelpers`
5. This sets the new `implementation` in the chain config.
6. Upgrade to the new implementation contract
   `node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum --upgrade`

## AxelarGasService and AxelarDepositService

1. Run the following depending on the service,
   `node evm/deploy-upgradable.js -e testnet -n ethereum -c AxelarGasService`
2. Use the `--upgrade` flag to upgrade the contract instead

## InterchainTokenService

To test the Interchain Token Service deployment

```bash
node evm/deploy-its -e testnet -n ethereum -s [salt]
```

Change the `-s SALT` to derive a new address.

## Governance

A governance contract is used to manage some contracts such as the AxelarGateway, ITS, ITS Factory etc. The governance is controlled by the native PoS based governance mechanism of Axelar.

1. Generate the governance proposal for Axelar
```bash
node evm/governance.js -n [chain] --targetContractName AxelarGateway --action [action] --proposalAction schedule --date 2023-11-10T03:00:00 --file proposal.json
```
2. Submit the proposal on Axelar. A min deposit needs to be provided. This can be found via `axelard q gov params`, and `axelard q axelarnet params` (if a higher deposit override is set for the specific contract).
```bash
axelard tx gov submit-proposal call-contracts proposal.json --deposit [min-deposit]uaxl --from [wallet] --chain-id [chain-id] --gas auto --gas-adjustment 1.4 --node [rpc]
```
3. Ask validators and community to vote on the proposal
```bash
axelard tx gov vote [proposal-id] [vote-option] --from [wallet] --chain-id [chain-id] --node [rpc]
```
4. Once the proposal passes after the voting period, a GMP call is initiated from Axelar to the EVM Governance contract.
5. This should be handled by relayers has executed the corresponding GMP calls. If it's not executed automatically, you can find the EVM batch to the chain via Axelarscan, and get the command ID from the batch,and submit the proposal.
```bash
node evm/governance.js -n [chain] --targetContractName AxelarGateway --action [action] --proposalAction submit --date 2023-12-11T08:45:00 --commandId [commandId]
```
6. Wait for timelock to pass on the proposal
7. Execute the proposal
```bash
node evm/governance.js -n [chain] --targetContractName AxelarGateway --action upgrade --proposalAction execute
```
8. Verify the governance command went through correctly.

## Utilities

### Decode Multicall

To decode multicall data:

1. Create a variable in the `.env` file named `MULTICALL_DATA`
2. Set this variable equal to the multicall data that needs to be decoded
   Example: `MULTICALL_DATA=0x00000000000000000000...`
3. Run the command below in the terminal:
```bash
    node evm/its.js --action decodeMulticall -e testnet -n ethereum -y
```
4. The resulting decoded function calls and arguments will be printed to the console.
   Example output for multicall data with `deployInterchainToken` and `interchainTransfer` calls:
```
Function: deployInterchainToken
Arg names: salt, destinationChain, name, symbol, decimals, minter, gasValue
Arg values: 0x79d4bf58fff996a2ffaca4809382c4ddb24b53d6def5712c141e97a010f68178, Chain A, Token A, TKA, 18, 0x1234, 90,
Function: interchainTransfer
Arg names: tokenId, destinationChain, destinationAddress, amount, metadata, gasValue
Arg values: 0x848f254a0b936a6b704ad1dad4a2867638db919eb10e5354cf526cccbd9fbc24, Chain B, 0x1234567890, 100, 0x00000001, 90
```
Note: If any encoded functions are not recognized, they will be printed to the console as unrecognized:
```
Function: Unrecognized function call
```

## InterchainGovernance

To update the min deposit on Axelar with a param change proposal, you can generate the proposal via
`node evm/min-deposit-proposal.js -e mainnet -n all --deposit 1000000`

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
