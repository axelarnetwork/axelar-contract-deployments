# EVM deployment scripts

This folder contains deployment and operational scripts for various contracts.
By default the version of contracts specified in `package.json` will be used for deployment.

## Setup

`npm ci && npm run build`

Add the deployer private key in `.env` folder (see `.example.env` for reference).

## Deployer Factories

EVM contracts can be deployed using one of 3 supported deployment methods:

- `create`: Standard nonce based contract deployment
- `create2`: Contract deployment using `CREATE2` opcode, the deployed address is deterministic based on the sender address, contract bytecode, and the salt
- `create3`: Contract deployment using the CREATE3 technique, the deployed address is deterministic based only on the sender address, and the salt. The dependency on the contract bytecode is removed, but as a result, you can't trust that the contract bytecode is the same across chains.

A tutorial can be found [here](https://www.axelar.network/blog/same-address-cross-chain-tutorial).

Factories have already been deployed on Axelar connected EVM chains. You can deploy your own factories via the following:

```bash
ts-node evm/deploy-contract.js -c Create2Deployer -m create

ts-node evm/deploy-contract.js -c Create3Deployer -m create2
```

## Axelar Amplifier Gateway

Deploy the Axelar Amplifier Gateway contract. This is the required gateway contract for EVM chains connecting via Axelar's Amplifier protocol.

`ts-node evm/deploy-amplifier-gateway.js -e testnet -n ethereum`

For debugging, you can deploy a gateway with the wallet set as the signer using `--keyID`. An owner can be set via `--owner` as well. It'll default to the deployer and can be transferred to governance later.

### Submit Amplifier Proofs

To submit proofs constructed on Amplifier to the gateway, use the following command:

```bash
ts-node evm/gateway.js --action submitProof --multisigSessionId [session id]
```

## Axelar Gateway (legacy connection)

Deploy the original Axelar gateway contract for legacy consensus-based connection. Set the governance and mint limiter via the `--governance` and `--mintLimiter` flags.

`ts-node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum`

## Gateway Upgrade

1. When upgrading the gateway, the proxy contract will be reused.
2. Depending on the upgrade process, Axelar auth and token deployer helper contracts might be reused as well.
3. `ts-node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum --reuseProxy` OR
4. `ts-node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum --reuseProxy --reuseHelpers`
5. This sets the new `implementation` in the chain config.
6. Upgrade to the new implementation contract
   `ts-node evm/deploy-gateway-v6.2.x.js -e testnet -n ethereum --upgrade`

## AxelarGasService and AxelarDepositService

1. Run the following depending on the service,
   `ts-node evm/deploy-upgradable.js -e testnet -n ethereum -c AxelarGasService`
2. Use the `--upgrade` flag to upgrade the contract instead

## InterchainTokenService

To test the Interchain Token Service deployment

```bash
ts-node evm/deploy-its -e testnet -n ethereum -s '[salt]' --proxySalt 'v1.0.0' -m create2
```

Change the `-s SALT` to derive a new address. Production deployments use the release version, e.g. `v1.2.1`.
`proxySalt` is used to derive the same address as a deployment on an existing chain.

## Governance

A governance contract is used to manage some contracts such as the AxelarGateway, ITS, ITS Factory etc. The governance is controlled by the native PoS based governance mechanism of Axelar.

1. Generate the governance proposal for Axelar

```bash
ts-node evm/governance.js -n [chain] --targetContractName AxelarGateway --action [action] --proposalAction schedule --date 2023-11-10T03:00:00 --file proposal.json
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
ts-node evm/governance.js -n [chain] --targetContractName AxelarGateway --action [action] --proposalAction submit --date 2023-12-11T08:45:00 --commandId [commandId]
```

6. Wait for timelock to pass on the proposal
7. Execute the proposal

```bash
ts-node evm/governance.js -n [chain] --targetContractName AxelarGateway --action upgrade --proposalAction execute
```

8. Verify the governance command went through correctly.

## Utilities

### Decode Function Calldata

To decode function calldata:

1. Run the command below with the calldata being decoded

    ```bash
    ts-node evm/decode.js -c [contractName] --calldata [calldata]
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

### Top Up Accounts

To top up multiple accounts from a single wallet with native cryptocurrency or ERC20 tokens, you can use the following command:

```bash
ts-node evm/top-up.js [native|token] -n <chain> --target <target-balance> --threshold <threshold> --addresses-to-derive [number-of-accounts]
```

Example usage:

```bash
# For native crypto
ts-node evm/top-up.js native -n xrpl-evm -t 50 --threshold 0 --addresses-to-derive 3

# For ERC20 tokens
ts-node evm/top-up.js token -n xrpl-evm -t 10000 --threshold 0 --addresses-to-derive 3 --contract 0x02D0f033d365d0D1b6a4377bfc6cB9D87bE16Ab7 --decimals 0
```

You can use `--addresses-to-derive` option to derive multiple accounts from a mnemonic set in the `MNEMONIC` environment variable, or use `--addresses` to specify a list of comma separated addresses to top up, e.g.:

```
ts-node evm/top-up.js native -n xrpl-evm -t 50 --threshold 0 --addresses 0xf6387dd5E4b60C1FA8A8bD590B28747D09882c9A,0xBBF8B24c74e10fEd75D554a74dFDE5805C5d7Cb5,0x033349e44e11C2a17828eb88D994eC7DD18d2175
```

For ERC20 token top ups, you must specify the contract address using `--contract` option.

### Load Test

Before running a load test, you need to set up the environment variable `MNEMONIC` with a valid mnemonic phrase, it will be used to derive the accounts that will generate transactions during the load test. Then run the following command to start the test:

```bash
ts-node evm/load-test.js test --source-chain <source-chain> --destination-chain <destination-chain> --time <minutes> --destination-address <destination-address> --token-id <its-token> --addresses-to-derive <number-of-accounts> --transfer-amount <amount>
```

Example usage:

```bash
ts-node evm/load-test.js test --source-chain xrpl-evm --destination-chain xrpl --time 60 --destination-address 0x7277577142334d3352694c634c724c6d754e34524e5964594c507239544e38483143 --token-id 0xba5a21ca88ef6bba2bfff5088994f90e1077e2a1cc3dcc38bd261f00fce2824f --addresses-to-derive 15 --transfer-amount 0.000001
```

The above example command will derive 15 accounts from the mnemonic set in the MNEMONIC environment variable, and each account will transfer 0.000001 tokens to the destination address in parallel continuosly for 60 minutes.

The transactions generated will be saved one per line in the file specified by the `--output` option, which defaults to `/tmp/load-test.txt`.

To verify the results of the load test, you can use the following command:

```bash
ts-node evm/load-test.js verify
```

The verification will read the transactions generated by the load test from the file specified by the `--input-file` option, which defaults to `/tmp/load-test.txt`, and check their status using the Axelarscan API.

The file must contain one transaction per line using the following format:

```
TRANSACTION_HASH[ : STATUS]
```

example:

```
0x3089cd1d913e901a3616f57446697bf92222e5edb2462aec0c7e0d29e152281a
0x4f0eb2154d193b751c1c2770095b971b30970416ed8abd733032e5d05b4a67dc
0x71374c24d47297512fe9e3480389a5ae2c7317ce73ca54ea21e6177a72bac91c : executing
0x3f6befb17a60e9aceee3d96d0c6d1d5e7eac3ead999be6279b6c8633c6deaf64 : error: RPC Call Failed: Transaction failed: tefNO_TICKET: Ticket is not in ledger.
```

The result of the verification will be saved in the files specified by the `--success-output`, `--fail-output`, and `--pending-output` options, which defaults to `/tmp/load-test-success.txt`, `/tmp/load-test-fail.txt`, and `/tmp/load-test-pending.txt` respectively.

In case the transaction failed or is still pending, the status or error message returned by the Axelarscan API will be appended to the transaction in the file. E.g.:

```
0x71374c24d47297512fe9e3480389a5ae2c7317ce73ca54ea21e6177a72bac91c : executing
```

If the verification is interrupted, you can resume from the last verified transaction number by specifying the `--resume-from` option, this will append to the output files instead of overwriting them. Example:

```bash
ts-node evm/load-test.js verify --resume-from 542
```

## InterchainGovernance

To update the min deposit on Axelar with a param change proposal, you can generate the proposal via
`ts-node evm/min-deposit-proposal.js -e mainnet -n all --deposit 1000000`

## Mock Deployment of Contracts

Test mock deployment of contracts using the `contracts-deployment-test.js` script:

```bash
ts-node evm/contracts-deployment-test.js -e <environment> -n <chainNames>
```

For example, to deploy contracts on the Famtom chain in the testnet environment:

```bash
ts-node evm/contracts-deployment-test.js -e testnet -n fantom
```

The script also supports optional flag parameters -y and --deployDepositService, which can also be specified in a .env file under the variables YES and DEPLOY_DEPOSIT_SERVICE.

Example with optional flags

```bash
ts-node evm/contracts-deployment-test.js -e testnet -n fantom -y --deployDepositService
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
ts-node evm/verify-contract.js -e mainnet -n [chain] -c AxelarGateway --dir /path/to/axelar-cgp-solidity
```

Verify Axelar wrapped tokens deployed via the gateway (`BurnableMintableCappedERC20` contract) on the chain in appropriate environment. The address will be retrieved from the gateway by default but can be provided explicitly.

```bash
ts-node evm/verify-contract.js -e mainnet -n [chain] -c BurnableMintableCappedERC20 --dir /path/to/axelar-cgp-solidity --args axlUSDC
```

Verify TokenManagerProxy contract for ITS. `--tokenId` must be specified and `--minter` can be optionally specified (otherwise will default to `0x`).

```bash
ts-node evm/verify-contract.js -e [env] -n [chain] -c TokenManagerProxy --dir /path/to/interchain-token-service --tokenId [tokenId]
```

## Verify Token Ownership requests

Download the pending requests [spreadsheet](https://docs.google.com/spreadsheets/d/1zKH1DINTiz83iXbbZRNRurxxZTaU0r5JS4A1c8b9-9A/edit?resourcekey=&gid=1705825087#gid=1705825087) into a csv format.

`ts-node evm/check-ownership-request.js -f sheet_path.csv`

## Verify AxelarAmplifierGateway contract.

`--address` can be optionally specified (otherwise will default to the value from config).

1. First clone the `axelar-gmp-sdk-solidity` repo: `git clone git@github.com:axelarnetwork/axelar-gmp-sdk-solidity.git`
2. Checkout the branch or commit from where the contract was deployed: `git checkout <branch_name>`
3. Run `npm ci && npm run build`
4. Create a keys.json file in root of the folder and add in it: `{"chains": {"<chain_name>>": {"api": "API_KEY"}}}`

```bash
ts-node evm/verify-contract.js -e [env] -n [chain] -c AxelarAmplifierGateway --dir /path/to/axelar-gmp-sdk-solidity
```

#### Help

To get details of options provided in the command run:

```bash
ts-node evm/verify-contract.js --help
```

## Interchain Token Service

### Link Token

#### Legacy custom ITS tokens

Custom tokens that have already registered with ITS (via `deployTokenManager`) prior to ITS v2.1.0 release can continue being linked to new chains via the following approach. However, we do recommend registering them. Token manager type should be passed in via `--type` flag (e.g. `MINT_BURN`).

```bash
ts-node evm/its.js link-token --salt [deploy-salt] [token-id] [destination-chain] [token-address] [type] [operator]
```

The raw `bytes32` salt can be provided via `--rawSalt [raw-salt]` instead of hashing the provided salt string.
