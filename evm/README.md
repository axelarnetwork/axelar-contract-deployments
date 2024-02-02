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
    node evm/decode.js -c CONTRACT_NAME --calldata [calldata]
    ```

2. The resulting decoded function call and arguments will be printed to the console.
   Example output for multicall data with `deployInterchainToken` and `interchainTransfer` calls:

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

## Contract Verification

### Prerequisites

-   Clone the repo containing the contract source code.

```bash
git clone https://github.com/axelarnetwork/axelar-cgp-solidity.git
```

-   Checkout to the version of contracts to verify in the directory provided to the command before compiling artifacts used by the command.

```bash
git checkout vX.Y.Z

npm ci

npm run build
```

-   Update `.hardhat.config.js` to have `chains` and `keys` to point to the current repo.

```javascript
const chains = require(`../axelar-contract-deployments/axelar-chains-config/info/${env}.json`);
const keys = readJSON(`../axelar-contract-deployments/keys.json`);
```

-   `keys.json` is expected to be in the format described [here](./.example.keys.json).
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

## Combine Validator Signatures

The following script is for emergency situations where we need to ask validators to manually sign gateway approvals. These validator signatures must be collected and stored within separate files in the signatures folder in the root directory. The script will then combine these signatures into a gateway batch and optionally execute this batch on the Axelar gateway contract on the specified chain.

Below are instructions on how to utilize this script:

1. Construct the gateway batch data by passing in the payload hash gotten from running the `governance.js` script.

    ```bash
    node evm/combine-signatures.js -e [ENV] -n [CHAIN_NAME] --action createBatchData --payloadHash [PAYLOAD_HASH]
    ```

    Note: Other gateway approval variables have default values but can be overriden via CLI options

    - sourceChain: 'Axelarnet'
    - sourceAddress: 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj'
    - contractAddress = interchain governance address
    - commandId = random bytes32 value

2. Running the command above will output the raw batch data, the data hash, and the vald sign command that should be used to generate validator signatures. Here is an example output:

    ```bash
    axelard vald-sign evm-ethereum-2-3799882 [validator-addr] 0x2716d6d6c37ccfb1ea1db014b175ba23d0f522d7789f3676b7e5e64e4322731
    ```

    The validator address should be entered dynamically based on which validator is signing.

3. Connect to AWS and switch to the correct context depending on which environment validators will be signing in.

4. Get all the namespaces within this context:

    ```bash
    kubectl get ns
    ```

5. List pods for each validator namespace. Example for stagenet validator 1:

    ```bash
    kubectl get pods -n stagenet-validator1
    ```

6. Exec into the validator pod. Example for stagenet validator 1:

    ```bash
    kubectl exec -ti axelar-core-node-validator1-5b68db5c49-mf5q6 -n stagenet-validator1 -c vald -- sh
    ```

7. Within the validator container, run the vald command from step 2 with the correct validator address. Validator addresses can be found by appending this endpoint `/axelar/multisig/v1beta1/key?key_id=[KEY_ID]` to the Axelar lcd url in the chains config file. Running this command should result in a JSON output containing the signature, similar to the example below:

    ```json
    {
        "key_id": "evm-ethereum-2-3799882",
        "msg_hash": "9dd126c2fc8b8a29f92894eb8830e552b7caf1a3c25e89cc0d74b99bc1165039",
        "pub_key": "030d0b5bff4bff1adb9ab47df8c70defe27bfae84b12b77fac64cdcc2263e8e51b",
        "signature": "634cb96060d13c083572d5c57ec9f6441103532434824a046dec4e8dfa0645544ea155230656ee37329572e1c52df1c85e48f0ef5f283bc7e036b9fd51ade6661b",
        "validator": "axelarvaloper1yfrz78wthyzykzf08h7z6pr0et0zlzf0dnehwx"
    }
    ```

8. Create a signatures folder in the root directory of this repository. Repeat step 7 until enough signatures have been generated to surpass the threshold weight for the current epoch (threshold weight can be found at the same endpoint as validator addresses). Store each signature JSON in a separate file within the signatures folder, filenames do not have to follow any specific convention.

9. Run the construct batch command passing in the raw batch data logged in step 1. This will generate the input bytes that can be passed directly into the execute command on Axelar gateway. You can also optionally pass the `--execute` command which will automatically execute the batch on the Axelar gateway contract.

    ```bash
    node evm/combine-signatures.js -e [ENV] -n [CHAIN_NAME] --action constructBatch --batchData [RAW_BATCH_DATA]
    ```

    Note: This step will generate the proof and validate it against the Axelar auth module before printing out the input bytes.
