# Stellar deployments

## Installation

Install `libsodium` for faster signing operations optionally. Install npm dependencies afterwards.

```sh
brew install libsodium

npm ci && npm run build
```

Install Stellar CLI

```bash
cargo install --locked stellar-cli --features opt
```

Add Soroban network in the config

```bash
stellar network add \
  --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

Create a new Stellar keypair

```bash
# Generate keypair
ts-node stellar/generate-keypair.js
```

Set `PRIVATE_KEY` in `.env` to the above value.

Testnet funds can be obtained via [this link](https://ftl.ai/) or using the `faucet.js` script:

```bash
ts-node stellar/faucet.js --recipient <address>
```

Send tokens (if needed)

```bash
ts-node stellar/send-tokens.js --amount <amount> --recipients <recipients>
```

## Deployments

Setup

1. Checkout the axelar-amplifier-stellar repo.
2. Compile the Stellar wasm contracts

```bash
cargo build
cargo test
stellar contract build
```

3. Optimize the contracts

```bash
./optimize.sh
```

### Gateway

Deploy the gateway contract

```bash
ts-node stellar/deploy-contract.js deploy AxelarGateway --version X.Y.Z
```

Provide `--estimate-cost` to show the gas costs for the initialize transaction instead of executing it.

### Operators

```bash
ts-node stellar/deploy-contract.js deploy AxelarOperators --version X.Y.Z
```

### Gas Service

```bash
ts-node stellar/deploy-contract.js deploy AxelarGasService --version X.Y.Z
```

### Interchain Token Service

```bash
ts-node stellar/deploy-contract.js deploy InterchainTokenService --version X.Y.Z
```

### Axelar Example

```bash
ts-node stellar/deploy-contract.js deploy AxelarExample --version X.Y.Z
```

## Upgrades

To facilitate contract upgrades, the `Upgrader` contract needs to be deployed first.

```bash
ts-node stellar/deploy-contract.js deploy Upgrader --version X.Y.Z
```

After the `Upgrader` is deployed, any other instantiated contract can be upgraded by calling the `upgrade` function

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --artifact-path ./axelar-amplifier-stellar/target/wasm32-unknown-unknown/release/<CONTRACT_NAME>.optimized.wasm --version <NEW_VERSION> --migration-data <MIGRATION_DATA>
```

where `<CONTRACT_NAME>` is the name of the contract to be upgraded and `--artifact-path` points to the upgraded bytecode. As a sanity check, `<NEW_VERSION>` must match the version number defined by the provided bytecode, so upgrading to the wrong version can be prevented. `<MIGRATION_DATA>` is the json encoded data that will be passed to the contract's `migrate` function. If the flag is not provided, the default value `()` will be used, meaning that the migration data is of type `void`. The easiest way to generate the json data for complex types is to instantiate the rust type the contract expects and then use `serde_json::to_string` to convert it to json.

Note: The `--artifact-path` flag is optional, so long as the `--version` flag is provided (and that version's wasm is present in R2 for download).

#### Example `MIGRATION_DATA` Type Input

For no migration data, omit the `--migration-data` flag, or pass `'()'` for the data.

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --version <NEW_VERSION>
```

or

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --version <NEW_VERSION> --migration-data '()'
```

For migration data of type `String`, omit the `--migration-data` flag and pass the string directly.

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --version <NEW_VERSION> --migration-data 'my string'
```

For migration data of type `Vec<Address>`, omit the `--migration-data` flag and pass the array as such:

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --version <NEW_VERSION> --migration-data '["GAA...", "GAB..."]'
```

For migration data of type `u64`, omit the `--migration-data` flag and pass the number directly.

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --version <NEW_VERSION> --migration-data 1234567890
```

For migration data of type `bool`, omit the `--migration-data` flag and pass the boolean directly:

```bash
ts-node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --version <NEW_VERSION> --migration-data true
```

## Uploads

In order to upload contracts directly to the Stellar network, use the following commands:

```bash
ts-node stellar/deploy-contract.js upload <CONTRACT_NAME> --version <NEW_VERSION>
```

```bash
ts-node stellar/deploy-contract.js upload <CONTRACT_NAME> --artifact-path ./axelar-amplifier-stellar/target/wasm32-unknown-unknown/release/<CONTRACT_NAME>.optimized.wasm
```

---

## Contract Interaction

Soroban contracts can be interacted directly via the CLI as well. See the help text for individual contract cmds as follows.

```bash
stellar contract invoke --network testnet --id [contract-address] --source-account wallet -- --help
```

### Gateway

To get help on the gateway commands, run:

```bash
ts-node stellar/gateway.js --help
```

#### Call contract

```bash
ts-node stellar/gateway.js call-contract [destination-chain] [dstination-address] [payload]

# Example
ts-node stellar/gateway.js call-contract avalanche 0x4F4495243837681061C4743b74B3eEdf548D56A5 0x1234
```

#### Submit multisig prover proof

Submit a proof constructed on Amplifier to the Stellar gateway contract.

```bash
ts-node stellar/gateway.js submit-proof [multisig-session-id]
```

#### Approve messages

A message approval can be submitted to the gateway contract for a test deployment where the wallet is the signer on the gateway. Setting `[destination-address]` to `wallet` will use the wallet address as the destination.

```bash
ts-node stellar/gateway.js approve [source-chain] [message-id] [source-address] [destination-address] [payload]
```

#### Validate messages

An approved message can be validated by the gateway contract for a test deployment as follows:

```bash
ts-node stellar/gateway.js validate-message [source-chain] [message-id] [source-address] [payload]
```

#### Rotate signers

A signer rotation can be submitted to the gateway contract. Use `--current-nonce` to override the default current nonce set for subsequent rotations. Skip `--signers` to rotate to the Amplifier verifier set registered in the prover contract.

```bash
ts-node stellar/gateway.js rotate --new-nonce test --signers wallet
ts-node stellar/gateway.js rotate --new-nonce test2 --current-nonce test --signers wallet
```

#### Execute

```bash
ts-node stellar/gateway.js execute [source-chain] [message-id] [source-address] [destination-address] [payload]
```

### Interchain Token Service

_Note_: Stellar ITS runs only in Hub mode. P2P connections are not supported. Therefore, rather than setting trusted ITS addresses, we set trusted chains (chains which are also registered with ITS Hub). The ITS Hub chain (axelar) itself is not a valid source/destination for direct ITS messages and so shouldn't be set as a trusted chain. All ITS messages must be sent to and received from the ITS Hub.

#### Add Trusted Chains

```bash
ts-node stellar/its.js add-trusted-chains <sourceChain> <sourceChain2> ...

# Example
ts-node stellar/its.js add-trusted-chains all
ts-node stellar/its.js add-trusted-chains avalanche sui
```

#### Remove Trusted Chains

```bash
ts-node stellar/its.js remove-trusted-chains <sourceChain> <sourceChain2> ...
```

#### Deploy Interchain Token

```bash
ts-node stellar/its.js deploy-interchain-token [name] [symbol] [decimal] [salt] [initial-supply]
```

#### Deploy Remote Interchain Token

```bash
ts-node stellar/its.js deploy-remote-interchain-token [salt] [destination-chain] --gas-amount [amount]
```

#### Register Canonical Token

```bash
ts-node stellar/its.js register-canonical-token [token-address]
```

#### Deploy Remote Canonical Token

```bash
ts-node stellar/its.js deploy-remote-canonical-token [token-address] [destination-chain] --gas-amount [amount]
```

#### Interchain Transfer

```bash
ts-node stellar/its.js interchain-transfer [token-id] [destination-chain] [destination-address] [amount] --data [data] --gas-amount [amount]
```

#### Execute

```bash
ts-node stellar/its.js execute [source-chain] [message-id] [source-address] [payload]
```

#### Get Flow Limit

```bash
ts-node stellar/its.js flow-limit [token-id]

# Example
ts-node stellar/its.js flow-limit 0x3e818f44d754748c2e7f59cfff8c34125884121fada921a31dcf383994eec1c5
```

#### Set Flow Limit

```bash
ts-node stellar/its.js set-flow-limit [token-id] [flow-limit]

# Example
ts-node stellar/its.js set-flow-limit 0x3e818f44d754748c2e7f59cfff8c34125884121fada921a31dcf383994eec1c5 1000000
```

#### Remove Flow Limit
```bash
ts-node stellar/its.js remove-flow-limit [token-id]

# Example
ts-node stellar/its.js remove-flow-limit 0x3e818f44d754748c2e7f59cfff8c34125884121fada921a31dcf383994eec1c5
```

## TTL extension and state archival recovery

All Soroban storage entries, including contract instances, have a 'time to live' (`ttl`) after which entries will be archived and no longer accessible until restored. The following commands can be used to extend `ttl` or restore archived contract instances.

Get the ttl of a contract instance:

```bash
ts-node stellar/contract.js get-ttl [contract-name]
```

Extend the ttl of a contract instance:

```bash
ts-node stellar/contract.js extend-instance [contract-name]

# Defaults to maximum extension amount. To specify the number of ledgers to extend:
ts-node stellar/contract.js extend-instance [contract-name] --extend-by [ledgers]
```

Restore an archived contract instance

```bash
ts-node stellar/contract.js restore-instance [contract-name]
```

### Example

#### GMP - Send Command (Outgoing)

```bash
ts-node stellar/gmp.js send [destination-chain] [destination-address] [payload] --gas-amount [amount]

# Example
ts-node stellar/gmp.js send avalanche 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

#### GMP - Execute Command (Incoming)

Note that approving the incoming message from the gateway needs to be done to execute it.

```bash
ts-node stellar/gmp.js [source-chain] [message-id] [source-address] [payload]

# Example
ts-node stellar/gmp.js execute avalanche '0x0bcbbfc9b006db6958f3fce75f11fdc306b45e8e43396211f414f40d2d6db7c5-0' 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

## Pausable Contract Functionality

Here is how to manage the pausable state of a Stellar contract. You can check if the contract is paused, pause the contract, or unpause the contract.

#### Usage

To use this script, run the following command with the appropriate options:

```bash
ts-node stellar/contract.js [action] [contract-name]
```

#### Options

- `[action]` can be one of the following:

    - `pause`: Pause the contract
    - `paused`: Check if the contract is paused
    - `unpause`: Unpause the contract

- `[contract-name]`: The name of the contract to interact with. This option is mandatory.

#### Examples

Check if the contract is paused:

```bash
ts-node stellar/contract.js paused AxelarGateway
```

Pause the contract:

```bash
ts-node stellar/contract.js pause AxelarGateway
```

Unpause the contract:

```bash
ts-node stellar/contract.js unpause AxelarGateway
```

## Transfer Ownership or Operatorship of the Contract

#### Usage

Transfer the ownership of the contract:

```bash
ts-node stellar/contract.js transfer-ownership [contract-name] [new-owner]
```

Transfer the operatorship of the contract:

```bash
ts-node stellar/contract.js transfer-operatorship [contract-name] [new-operator]
```

## Deploy Stellar Asset Contract

The deploy-stellar-asset-contract command allows you to deploy a Stellar asset contract through the Token Utils contract. This creates a smart contract wrapper for an existing Stellar classic asset, enabling it to be used within the Stellar ecosystem. The command validates the asset parameters and returns the deployed contract's address.

#### Usage

```bash
ts-node stellar/token-utils.js deploy-stellar-asset-contract [asset-code] [issuer-address]
```

#### Parameters

- `<asset-code>`: The asset code for the Stellar asset (e.g., "USDC", "AQUA", "PEN")
- `<issuer-address>`: The Stellar address of the asset issuer

#### Example

```bash
ts-node stellar/token-utils.js deploy-stellar-asset-contract PEN GALVTUIOIAXUB7FHCUS4PFPMILNIGG4DW4S2MHMB2EG7URASFBR5H374
```
