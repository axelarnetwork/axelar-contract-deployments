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

`ts-node evm/deploy-amplifier-gateway.js`

For debugging, you can deploy a gateway with the wallet set as the signer using `--keyID`. An owner can be set via `--owner` as well. It'll default to the deployer and can be transferred to governance later.

### Submit Amplifier Proofs

To submit proofs constructed on Amplifier to the gateway, use the following command:

```bash
ts-node evm/gateway.js --action submitProof --multisigSessionId [session id]
```

## Axelar Gateway (legacy connection)

Deploy the original Axelar gateway contract for legacy consensus-based connection. Set the governance and mint limiter via the `--governance` and `--mintLimiter` flags.

`ts-node evm/deploy-gateway-v6.2.x.js`

## Gateway Upgrade

1. When upgrading the gateway, the proxy contract will be reused.
2. Depending on the upgrade process, Axelar auth and token deployer helper contracts might be reused as well.
3. `ts-node evm/deploy-gateway-v6.2.x.js --reuseProxy` OR
4. `ts-node evm/deploy-gateway-v6.2.x.js --reuseProxy --reuseHelpers`
5. This sets the new `implementation` in the chain config.
6. Upgrade to the new implementation contract
   `ts-node evm/deploy-gateway-v6.2.x.js --upgrade`

## AxelarGasService and AxelarDepositService

1. Run the following depending on the service,  
   `ts-node evm/deploy-upgradable.js -c AxelarGasService`
2. Use the `--upgrade` flag to upgrade the contract instead
3. To reuse the existing proxy, you can:
   - Deploy new implementation contract:
     ```bash
     ts-node evm/deploy-upgradable.js \
       -c AxelarGasService \
       -m create2 \
       --reuseProxy
     ```
   - Perform the upgrade using the stored implementation address:
     ```bash
     ts-node evm/deploy-upgradable.js \
       -c AxelarGasService \
       -m create2 \
       --upgrade
     ```

## InterchainTokenService

To test the Interchain Token Service deployment

```bash
ts-node evm/deploy-its -s '[salt]' --proxySalt 'v1.0.0' -m create2
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

## InterchainGovernance & AxelarServiceGovernance

Full docs can be found on [here](./docs/governance.md).
Detailed workflows are mentioned [here](./docs/governance-workflows.md).

## Contract Ownership Management

Full docs can be found [here](./docs/contract-ownership.md).

8. Verify the governance command went through correctly.

### InterchainTokenService owner commands (evm/its.js)

#### Set trusted chains
`ts-node evm/its.js set-trusted-chains <chain1> <chain2> ...`

#### Remove trusted chains
`ts-node evm/its.js remove-trusted-chains <chain1> <chain2> ... --yes`

#### Migrate interchain token
`ts-node evm/its.js migrate-interchain-token <tokenId> --yes`

*Note: add the following flags for operating via governance: `--governance --activationTime 2025-12-31T12:00:00 [--generate-only proposal.json]` and then submit the proposal

### Gateway operator commands (evm/gateway.js)

#### Rotate signers (emergency)
`ts-node evm/gateway.js --action rotateSigners --payload <payload> --proof <proof> --yes`

#### Transfer operatorship
`ts-node evm/gateway.js --action transferOperatorship --destination <gatewayAddress> --payload <calldata> --yes`

Other gateway actions remain in `evm/gateway.js`; use `--action` accordingly.

### Operators script (evm/operators.js)

#### Add operator
`ts-node evm/operators.js --action addOperator --operator <addr> --yes`

#### Remove operator
`ts-node evm/operators.js --action removeOperator --operator <addr> --yes`

#### Transfer ownership
`ts-node evm/operators.js --action transferOwnership --newOwner <addr> --yes`

#### Propose ownership
`ts-node evm/operators.js --action proposeOwnership --newOwner <addr> --yes`

#### Execute contract (operators role)
`ts-node evm/operators.js --action executeContract --target <addr> --calldata <0x...> --nativeValue <wei> --yes`

### AxelarGasService commands (evm/gas-service.js)

#### Estimate gas fee
`ts-node evm/gas-service.js --action estimateGasFee --destinationChain <chain> --destinationAddress <addr> --payload <0x...> --executionGasLimit <gas> [--isExpress]`

#### Update gas info across chains
`ts-node evm/gas-service.js --action updateGasInfo --chains <chain1> <chain2> ...`

Note: For upgrades, continue to use governance flows; operational actions run via this script.

### InterchainTokenService operator commands (evm/its.js)
Note: For upgrades, continue to use governance flows; operational actions run via this script.

#### Transfer operatorship
`ts-node evm/its.js transfer-operatorship <operator> --yes`

#### Propose operatorship
`ts-node evm/its.js propose-operatorship <operator> --yes`

*Note: add the following flags for operating via governance: `--governance --activationTime 2025-12-31T12:00:00 [--generate-only proposal.json]` and then submit the proposal

### AxelarServiceGovernance (operator) extensions

`AxelarServiceGovernance` extends `InterchainGovernance` with operator approval functionality that can bypass timelock.

- Full CLI + examples: [docs/governance.md](./docs/governance.md)
- End-to-end workflows: [docs/governance-workflows.md](./docs/governance-workflows.md)
- Amplifier (no relayers / manual proof): [docs/amplifier-governance.md](./docs/amplifier-governance.md)

**Activation time:** use `YYYY-MM-DDTHH:mm:ss` (UTC) or `0` (immediate; min delay is enforced on-chain).

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


### Interchain Token Deploy Salt

Computes the deploy salt for an interchain token.

```bash
ts-node evm/interchainTokenFactory.js interchain-token-deploy-salt --deployer <deployer>  --chainNames <chain_name> --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js interchain-token-deploy-salt --deployer 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f  --chainNames ethereum-sepolia --env testnet --salt 0x4ab94b9bf7e0a1c793d3ff3716b18bb3200a224832e16d1d161bb73a698c8253
```

### Canonical Interchain Token Deploy Salt

Computes the deploy salt for a canonical interchain token.

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-deploy-salt --tokenAddress <token_address> --chainNames <chain_name>  --env <env>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-deploy-salt --tokenAddress 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1 --chainNames ethereum-sepolia  --env testnet
```

### Canonical Interchain Token Id

Computes the ID for a canonical interchain token based on its address.

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-id --tokenAddress <token_address> --chainNames <chain_name>  --env <env>

```

Example:

```bash
ts-node evm/interchainTokenFactory.js canonical-interchain-token-id --tokenAddress 0x8A80b16621e4a14Cb98B64Fd2504b8CFe0Bf5AF1 --chainNames ethereum-sepolia  --env testnet
```

### Interchain Token Id

Computes the ID for an interchain token based on the deployer and a salt.

```bash
ts-node evm/interchainTokenFactory.js interchain-token-id --deployer <deployer> --chainNames <chain_name> --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js interchain-token-id --deployer 0x312dba807EAE77f01EF3dd21E885052f8F617c5B --chainNames avalanche --env testnet --salt 0x48d1c8f6106b661dfe16d1ccc0624c463e11e44a838e6b1f00117c5c74a2cd82
```

### Deploy Interchain Token

Creates a new token and optionally mints an initial amount to a specified minter

```bash
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name <name> --symbol <symbol> --decimals <decimals> --initialSupply <initialSupply> --minter <minter>  --chainNames <chain_name> --env <env> --salt <salt>
```


Example:

```bash
ts-node evm/interchainTokenFactory.js deploy-interchain-token --name Test_Token --symbol TT --decimals 18 --initialSupply 12345 --minter 0x312dba807EAE77f01EF3dd21E885052f8F617c5B  --chainNames ethereum-sepolia --env testnet --salt 0x7abda5c65fc2720ee1970bbf2a761f6d5b599065283d3c184cb655066950e51a
```


### Deploy Remote Interchain Token

Deploys a remote interchain token on a specified destination chain. No additional minter is set on the deployed token.

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token --destinationChain <destination_chain> --chainNames <chain_name>  --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-interchain-token --destinationChain Avalanche  --chainNames ethereum-sepolia  --env testnet --salt 0x7abda5c65fc2720ee1970bbf2a761f6d5b599065283d3c184cb655066950e51a
```


### Register Canonical Interchain Token

Registers a canonical token as an interchain token and deploys its token manager.

```bash
ts-node evm/interchainTokenFactory.js register-canonical-interchain-token --tokenAddress <token_address> --chainNames <chain_name> --env <env>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js register-canonical-interchain-token --tokenAddress 0xff0021D9201B51C681d26799A338f98741fBBB6a --chainNames ethereum-sepolia --env testnet
```

### Deploy Remote Canonical Interchain Token

Deploys a canonical interchain token on a remote chain.

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token --tokenAddress <token_address> --destinationChain <destination_chain> --chainNames <chain_name> --env <env>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js deploy-remote-canonical-interchain-token --tokenAddress 0x4a895FB659aAD3082535Aa193886D7501650685b --destinationChain Avalanche --chainNames ethereum-sepolia --env testnet
```

### Register Custom Token

Register an existing ERC20 token under a `tokenId` computed from the provided `salt`.

```bash
ts-node evm/interchainTokenFactory.js register-custom-token  --tokenAddress <token_address> --tokenManagerType <token_manager_type> --operator <operator> --chainNames <chain_name> --env <env> --salt <salt>
```


Example:

```bash
ts-node evm/interchainTokenFactory.js register-custom-token --tokenAddress 0x0F6814301C0DA51bFddA9D2A6Dd877950aa0F912 --tokenManagerType 4 --operator 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f --chainNames ethereum-sepolia --env testnet --salt 0x3c39e5b65a730b26afa28238de20f2302c2cdb00f614f652274df74c88d4bb50
```

Note:
Custom tokens that wish to utlize Mint/Burn token managers must implement the mint and burn interfaces to match:

```bash
mint(address to, uint256 amount);
```
```bash
burn(address from, uint256 amount);
```

### Link Token

Links a remote token on `destinationChain` to a local token corresponding to the `tokenId` computed from the provided `salt`.

```bash
ts-node evm/interchainTokenFactory.js link-token --destinationChain <destination_chain> --destinationTokenAddress <destination_token_address> --tokenManagerType <token_manager_type> --linkParams <link_params>  --chainNames <chain_name> --env <env> --salt <salt>
```

Example:

```bash
ts-node evm/interchainTokenFactory.js link-token --destinationChain Avalanche --destinationTokenAddress 0xB98cF318A3cB1DEBA42a5c50c365B887cA00133C --tokenManagerType 4 --linkParams 0x03555aA97c7Ece30Afe93DAb67224f3adA79A60f  --chainNames ethereum-sepolia --env testnet --yes --salt 0x3c39e5b65a730b26afa28238de20f2302c2cdb00f614f652274df74c88d4bb40
```
