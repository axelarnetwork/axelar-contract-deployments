# EVM deployment scripts

This folder contains deployment and operational scripts for various contracts.
By default the version of contracts specified in `package.json` will be used for deployment.

## Setup

`npm ci`

Add the deployer private key in `.env` folder (see `.example.env` for reference).

## Deployer Factories

EVM contracts can be deployed using one of 3 supported deployment methods:

- `create`: Standard nonce based contract deployment
- `create2`: Contract deployment using `CREATE2` opcode, the deployed address is deterministic based on the sender address, contract bytecode, and the salt
- `create3`: Contract deployment using the CREATE3 technique, the deployed address is deterministic based only on the sender address, and the salt. The dependency on the contract bytecode is removed, but as a result, you can't trust that the contract bytecode is the same across chains.

A tutorial can be found [here](https://www.axelar.network/blog/same-address-cross-chain-tutorial).

Factories have already been deployed on Axelar connected EVM chains. You can deploy your own factories via the following:

```bash
node evm/deploy-contract.js -c Create2Deployer -m create

node evm/deploy-contract.js -c Create3Deployer -m create2
```

## Axelar Amplifier Gateway

Deploy the Axelar Amplifier Gateway contract. This is the required gateway contract for EVM chains connecting via Axelar's Amplifier protocol.

`node evm/deploy-amplifier-gateway.js -e testnet -n ethereum`

For debugging, you can deploy a gateway with the wallet set as the signer using `--keyID`. An owner can be set via `--owner` as well. It'll default to the deployer and can be transferred to governance later.

### Submit Amplifier Proofs

To submit proofs constructed on Amplifier to the gateway, use the following command:

```bash
node evm/gateway.js --action submitProof --multisigSessionId [session id]
```

## Axelar Gateway (legacy connection)

Deploy the original Axelar gateway contract for legacy consensus-based connection. Set the governance and mint limiter via the `--governance` and `--mintLimiter` flags.

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
node evm/deploy-its -e testnet -n ethereum -s '[salt]' --proxySalt 'v1.0.0' -m create2
```

Change the `-s SALT` to derive a new address. Production deployments use the release version, e.g. `v1.2.1`.
`proxySalt` is used to derive the same address as a deployment on an existing chain.

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

### Decode Function Calldata

To decode function calldata:

1. Run the command below with the calldata being decoded

    ```bash
    node evm/decode.js -c [contractName] --calldata [calldata]
    ```

2. Example output for multicall data with `deployInterchainToken` and `interchainTransfer` calls. `contractName` can be `InterchainTokenService` or `InterchainTokenFactory` depending on which contract the ITS related method is for.

    ```
    Decoded calldata:
    Function: multicall
    Decoded multicall:
    Function: deployInterchainToken
    Arg names: salt, destinationChain, name, symbol, decimals, minter, gasValue
    Arg values: 0x79d4bf58fff996a2ffaca4809382c4ddb24b53d6def5712c141e97a010f68178, Chain A, Token A, TKA, 18, 0x1234, 90,
    Function: interchainTransfer
    Arg names: tokenId, destinationChain, destinationAddress, amount, metadata, gasValue
    Arg values: 0x848f254a0b936a6b704ad1dad4a2867638db919eb10e5354cf526cccbd9fbc24, Chain B, 0x1234567890, 100, 0x00000001, 90,
    Function: deployInterchainToken
    Arg names: salt, destinationChain, name, symbol, decimals, minter, gasValue
    Arg values: 0x2fd80269c9e4de03c2cd98afc956a4fed8d18e60428b79dcc9bf581cf8c63c1a, Chain B, Token B, TKB, 18, 0x5678, 120
    ```

    Example output for `deployInterchainToken` calldata:

    ```
    Decoded calldata:
    Function: deployInterchainToken
    Arg names: salt, destinationChain, name, symbol, decimals, minter, gasValue
    Arg values: 0x79d4bf58fff996a2ffaca4809382c4ddb24b53d6def5712c141e97a010f68178, Chain A, Token A, TKA, 18, 0x1234, 90
    ```

    Note: If any encoded functions are not recognized, they will be printed to the console as unrecognized:

    ```
    Function: Unrecognized function call
    ```

## InterchainGovernance

To update the min deposit on Axelar with a param change proposal, you can generate the proposal via
`node evm/min-deposit-proposal.js -e mainnet -n all --deposit 1000000`

## Mock Deployment of Contracts

Test mock deployment of contracts using the `contracts-deployment-test.js` script:

```bash
node evm/contracts-deployment-test.js -e <environment> -n <chainNames>
```

For example, to deploy contracts on the Famtom chain in the testnet environment:
```bash
node evm/contracts-deployment-test.js -e testnet -n fantom
```
The script also supports optional flag parameters -y and --deployDepositService, which can also be specified in a .env file under the variables YES and DEPLOY_DEPOSIT_SERVICE.

Example with optional flags
```bash
node evm/contracts-deployment-test.js -e testnet -n fantom -y --deployDepositService
```

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

Verify TokenManagerProxy contract for ITS. `--tokenId` must be specified and `--minter` can be optionally specified (otherwise will default to `0x`).

```bash
node evm/verify-contract.js -e [env] -n [chain] -c TokenManagerProxy --dir /path/to/interchain-token-service --tokenId [tokenId]
```

## Verify Token Ownership requests

Download the pending requests [spreadsheet](https://docs.google.com/spreadsheets/d/1zKH1DINTiz83iXbbZRNRurxxZTaU0r5JS4A1c8b9-9A/edit?resourcekey=&gid=1705825087#gid=1705825087) into a csv format.

`node evm/check-ownership-request.js -f sheet_path.csv`

## Verify AxelarAmplifierGateway contract.
`--address` can be optionally specified (otherwise will default to the value from config).

1. First clone the `axelar-gmp-sdk-solidity` repo: `git clone git@github.com:axelarnetwork/axelar-gmp-sdk-solidity.git`
2. Checkout the branch or commit from where the contract was deployed: `git checkout <branch_name>`
3. Run `npm ci && npm run build`
4. Create a keys.json file in root of the folder and add in it: `{"chains": {"<chain_name>>": {"api": "API_KEY"}}}`

```bash
node evm/verify-contract.js -e [env] -n [chain] -c AxelarAmplifierGateway --dir /path/to/axelar-gmp-sdk-solidity
```

#### Help

To get details of options provided in the command run:

```bash
node evm/verify-contract.js --help
```
