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

## AxelarTransceiver and ERC1967 Proxy Deployment

Note: You can deploy transceiver for any tokens/chains by providing the appropriate  `--transceiverPrefix`. For deployment purposes, we use `AxelarTransceiver` contract from [library](https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth.git). The deployment script saves the config under the full name of Transceiver contract (e.g., `LidoAxelarTransceiver`, etc)

### Prerequisites

AxelarTransceiver and ERC1967Proxy contract are compiled from the example-wormhole-axelar-wsteth repo. Build is generated using the following commands:

```bash
git clone https://github.com/wormhole-foundation/example-wormhole-axelar-wsteth.git
forge build --out out --libraries "lib/example-native-token-transfers/evm/src/libraries/TransceiverStructs.sol:TransceiverStructs:<$TRANSCEIVER_STRUCTS_ADDRESS>"
```

- Note: Pre-linked artifacts will be generated, i.e. TransceiverStructs library will be linked. This step is mandatory to deploy AxelarTransceiver contract.

### AxelarTransceiver Deployment

Please ensure you have generated pre-linked artifacts.

Set address of deployed `gmpManager` to the transceiver section in your chain config:

```json
"${TRANSCEIVER_PREFIX}AxelarTransceiver": {
  "gmpManager": "0x..."
}
```

To deploy an AxelarTransceiver contract, run:

```bash
ts-node evm/deploy-contract.js \
  -c AxelarTransceiver \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-wsteth/out/ \
  --transceiverPrefix $TRANSCEIVER_PREFIX
```

**Important**:

- **Use `create`** method to deploy, as deployer of transceiver will be used to initialize the contract, avoid using `create2` or `create3`
- **`--artifactPath` is required** for transceiver deployment
- **`--transceiverPrefix` is required** to differentiate multiple transceivers in config
- The GMP Manager address is automatically read from the chain config (`${TRANSCEIVER_PREFIX}AxelarTransceiver.gmpManager`) or can be manually provided via `--gmpManager` flag
- **Library Linking**: Pre-linked artifacts are generated and required libraries are already linked

The deployment script will:

- Validate the gateway, gas service, and GMP manager addresses from the chain configuration
- Deploy the contract with the correct constructor arguments
- Store configuration including gateway, gas service, and GMP manager addresses
- Verify the deployed contract state matches the original constructor arguments

#### Upgrade Transceiver

To upgrade an existing transceiver implementation, follow these steps:

##### Deploy New Implementation (Reuse Existing Proxy)

```bash
ts-node evm/deploy-contract.js \
  -c AxelarTransceiver \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-wsteth/out/ \
  --transceiverPrefix $TRANSCEIVER_PREFIX \
  --reuseProxy
```

##### Upgrade Proxy to Point to New Implementation

```bash
ts-node evm/deploy-contract.js \
  -c AxelarTransceiver \
  --artifactPath path/to/example-wormhole-axelar-wsteth/out/ \
  --transceiverPrefix $TRANSCEIVER_PREFIX \
  --upgrade
```

### ERC1967Proxy

The `deploy-contract.js` script supports deploying ERC1967Proxy contracts for any contract. Use the `--forContract` option to specify the full contract name like this:

```bash
ts-node evm/deploy-contract.js \
  -c ERC1967Proxy \
  -m create \
  --artifactPath path/to/example-wormhole-axelar-wsteth/out/ \
  --forContract `${TRANSCEIVER_PREFIX}AxelarTransceiver`
```

**Important**:

- **Use `create`** method to deploy for ERC1967Proxy of `${TRANSCEIVER_PREFIX}AxelarTransceiver`, as deployer will be used to initialize the contract
- **`--artifactPath` is required** for ERC1967Proxy deployment
- **Default deployment method is `create`** (standard nonce-based deployment)
- Use `-m create2` or `-m create3` for deterministic deployments if needed

The proxy deployment will:

- Use the implementation address from the specified contract's config
- Store the proxy address in the target contract's configuration
- Support custom initialization data via `--proxyData` (defaults to "0x")

### Transceiver Post-Deployment Operations

After deploying a transceiver contract, you can perform post-deployment operations using the `axelar-transceiver.ts` script:

```bash
# Initialize the transceiver contract
ts-node evm/axelar-transceiver.ts initialize --artifactPath path/to/example-wormhole-axelar-wsteth/out/ --transceiverPrefix $TRANSCEIVER_PREFIX

# Transfer pauser capability to a new address
ts-node evm/axelar-transceiver.ts transfer-pauser 0x... --artifactPath path/to/example-wormhole-axelar-wsteth/out/ --transceiverPrefix $TRANSCEIVER_PREFIX

# Set Chain ID mapping
ts-node evm/axelar-transceiver.ts set-axelar-chain-id <WormholeChainId> <AxelarChainName> <TransceiverAddress> --artifactPath path/to/example-wormhole-axelar-wsteth/out/ --transceiverPrefix $TRANSCEIVER_PREFIX
```

## Hyperliquid

The Hyperliquid chain uses a dual architecture block model with fast blocks (2 seconds, 2M gas limit) and slow blocks (1 minute, 30M gas limit). The `hyperliquid.js` script provides utilities to set an account to used a specific block size, to query the deployer address of an interchain token, and to update the deployer address of an interchain token. The supported commands are:

```bash
# Update block size
ts-node evm/hyperliquid.js update-block-size <small|big>

# Get token deployer
ts-node evm/hyperliquid.js deployer <token-id>

# Update token deployer
ts-node evm/hyperliquid.js update-token-deployer <token-id> <address>
```

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

## Interchain Token Factory

The Interchain Token Factory is responsible for deploying new interchain tokens and managing their token managers. It has the following functionality:

### Contract-Id

Getter for the contract id.

```bash
ts-node evm/interchainTokenFactory.js contract-id --chainNames <chain_name> --env <env> 
```

Example:

```bash
ts-node evm/interchainTokenFactory.js contract-id --chainNames avalanche --env testnet  
```

Example Response:

```bash
InterchainTokenFactory contract ID: 0x80547d63ed663962b99f8ed432bff3879a35b5418af92258aa171feef14cc3cc
```

### Interchain Token Deploy Salt

Computes the deploy salt for an interchain token.

```bash
ts-node evm/interchainTokenFactory.js interchain-token-deploy-salt <deployer>  --chainNames <chain_name> --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js interchain-token-deploy-salt 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f  --chainNames ethereum-sepolia --env testnet --salt 0x4ab94b9bf7e0a1c793d3ff3716b18bb3200a224832e16d1d161bb73a698c8253
```

Example Response:

```bash
interchainTokenDeploySalt for deployer 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f and deployment salt: 0x511cf8ffa226b88f7412a8d05960efcedb6526849eff5932bf063e008ece393b
```

### Canonical Interchain Token Deploy Salt

Computes the deploy salt for a canonical interchain token.

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-deploy-salt <token_address> --chainNames <chain_name>  --env <env>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-deploy-salt 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1 --chainNames ethereum-sepolia  --env testnet
```

Example Response:

```bash
canonicalInterchainTokenDeploySalt for token address: 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1: 0xe513f2267d22ff732afa2c2ffacbea3d620a1aaf56ff2da43364e50580a74db5
```

### Canonical Interchain Token Id

Computes the ID for a canonical interchain token based on its address.

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-id <token_address> --chainNames <chain_name>  --env <env>

```

Example:

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-id 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1 --chainNames ethereum-sepolia  --env testnet
```

Example Response:

```bash
canonicalInterchainTokenId for token address: 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1: 0x578a3eef4bdf76cad3cc2aba334341759c02a9c233dece18ed64ba32c2e0f67f
```

### Interchain Token Id

Computes the ID for an interchain token based on the deployer and a salt.

```bash
ts-node evm/interchainTokenFactory.js interchain-token-id <deployer> --chainNames <chain_name> --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js interchain-token-id 0x312dba807EAE77f01EF3dd21E885052f8F617c5B --chainNames avalanche --env testnet --salt 0x48d1c8f6106b661dfe16d1ccc0624c463e11e44a838e6b1f00117c5c74a2cd82
```

Example Response:
```bash
InterchainTokenId for deployer 0x312dba807EAE77f01EF3dd21E885052f8F617c5B and deployment salt: 0x28cdf5de538ba9ca6dde00c89f20e0de32f63c9a3052295cb162daf4cf3cb358: 0xd162c4aec6dca05d0c3be25937a4e8743c144000818f952b2199b29cd69e41c7
```

### Deploy Interchain Token

Creates a new token and optionally mints an initial amount to a specified minter

```bash
ts-node evm/interchainTokenFactory.js deploy-interchain-token <name> <symbol> <decimals> <initialSupply> <minter>  --chainNames <chain_name> --env <env> --salt <salt>
```


Example:

```bash
ts-node evm/interchainTokenFactory.js deploy-interchain-token Test_Token TT 18 12345 0x312dba807EAE77f01EF3dd21E885052f8F617c5B  --chainNames ethereum-sepolia --env testnet --salt 0x7abda5c65fc2720ee1970bbf2a761f6d5b599065283d3c184cb655066950e51a
```

Example Response:

```bash
Token address: 0x5330f9bA7F231F7fe1aC8b7e6bC880a4ebC7Ff8d
```

### Deploy Remote Interchain Token

Deploys a remote interchain token on a specified destination chain. No additional minter is set on the deployed token.

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token <destination_chain> <gas_value>  --chainNames <chain_name>  --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token Avalanche 10000000000000000  --chainNames ethereum-sepolia  --env testnet --salt 0x7abda5c65fc2720ee1970bbf2a761f6d5b599065283d3c184cb655066950e51a
```


### Register Canonical Interchain Token

Registers a canonical token as an interchain token and deploys its token manager.

```bash
ts-node evm/interchainTokenFactory.js register-canonical-interchain-token <token_address> --chainNames <chain_name> --env <env>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js register-canonical-interchain-token 0xff0021D9201B51C681d26799A338f98741fBBB6a --chainNames ethereum-sepolia --env testnet
```

Example Response:

```bash
register-canonical-interchain-token tx: 0x18a5cb3a1095b0963fa3110ee9efce9c0640fbb9a4338d6aa12f788a43ffa4aa
```


### Deploy Remote Canonical Interchain Token

Deploys a canonical interchain token on a remote chain.

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token <token_address> <destination_chain> <gas_value> --chainNames <chain_name> --env <env>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token 0x4a895FB659aAD3082535Aa193886D7501650685b Avalanche 100000000000 --chainNames ethereum-sepolia --env testnet
```

Example Response:

```bash
deploy-remote-canonical-interchain-token tx: 0xf23e2b939c2af373bb4db004f96cacbfcbdb0e4c6acfa97b42ede309cbfbca65
```


### Register Custom Token

Register an existing ERC20 token under a `tokenId` computed from the provided `salt`.

```bash
ts-node evm/interchainTokenFactory.js register-custom-token  <token_address> <token_manager_type> <operator> --chainNames <chain_name> --env <env> --salt <salt>
```


Example:

```bash
ts-node evm/interchainTokenFactory.js register-custom-token 0x0F6814301C0DA51bFddA9D2A6Dd877950aa0F912 4 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --chainNames ethereum-sepolia --env testnet --salt 0x3c39e5b65a730b26afa28238de20f2302c2cdb00f614f652274df74c88d4bb50
```

Example Response: 

```bash
register-custom-token tx: 0xeab6d934a487e1242d8fe2704bf59e59e456b9db2d736d98dca5a54fc93a2877
```

### Link Token

Links a remote token on `destinationChain` to a local token corresponding to the `tokenId` computed from the provided `salt`.

```bash
ts-node evm/interchainTokenFactory.js link-token <destination_chain> <destination_token_address> <token_manager_type> <link_params> <gas_value>  --chainNames <chain_name> --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js link-token Avalanche 0xB98cF318A3cB1DEBA42a5c50c365B887cA00133C 4 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f 1000000  --chainNames ethereum-sepolia --env testnet --yes --salt 0x3c39e5b65a730b26afa28238de20f2302c2cdb00f614f652274df74c88d4bb40
```

Example Response:

```bash
link-token tx: 0xbc5feb02b1af05c8ad2aceae63edafd02cd74d8cb4976181e091d4c8cacd2505
```
