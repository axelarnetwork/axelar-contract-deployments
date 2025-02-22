# Stellar deployments

## Installation

Install `libsodium` for faster signing operations optionally. Install npm dependencies afterwards.

```sh
brew install libsodium

npm ci
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
stellar keys generate wallet --network testnet

# Address
stellar keys address wallet

# Get private key
stellar keys show wallet
```

Set `PRIVATE_KEY` in `.env` to the above value.

Testnet funds can be obtained via [this link](https://ftl.ai/) or using the `faucet.js` script:

```bash
node stellar/faucet.js --recipient <address>
```

## Deployments

Setup

1. Checkout the axelar-cgp-stellar repo.
2. Compile the Stellar wasm contracts

```bash
cargo build
stellar contract build
```

3. Optimize the contracts

```bash
./optimize.sh
```

### Gateway

Deploy the gateway contract

```bash
node stellar/deploy-contract.js deploy axelar_gateway --version v1.0.0
```

Provide `--estimate-cost` to show the gas costs for the initialize transaction instead of executing it.

### Operators

```bash
node stellar/deploy-contract.js deploy axelar_operators --version v1.0.0
```

### Gas Service

```bash
node stellar/deploy-contract.js deploy axelar_gas_service --version v1.0.0
```

### Interchain Token Service

Deploy Interchain Token and Token Manager wasm first.

```bash
node stellar/deploy-contract.js deploy interchain_token --version v1.0.0
node stellar/deploy-contract.js deploy token_manager --version v1.0.0
node stellar/deploy-contract.js deploy interchain_token_service --version v1.0.0
```

### Example

Note that example contract should use `--wasm-path` option to deploy contract

```bash
node stellar/deploy-contract.js deploy example --wasm-path ../axelar-cgp-stellar/target/wasm32-unknown-unknown/release/stellar_example.optimized.wasm
```

### Contract upgrades

To facilitate contract upgrades, the `upgrader` contract needs to be deployed first.

```bash
node stellar/deploy-contract.js deploy upgrader --version v1.0.0
```

After the `upgrader` is deployed, any other instantiated contract can be upgraded by calling the `upgrade` function

```bash
node stellar/deploy-contract.js upgrade <CONTRACT_NAME> --wasm-path ../axelar-cgp-stellar/target/wasm32-unknown-unknown/release/<CONTRACT_NAME>.optimized.wasm --new-version <NEW_VERSION> --migration-data <MIGRATION_DATA>
```

where `<CONTRACT_NAME>` is the name of the contract to be upgraded and `--wasm-path` points to the upgraded bytecode. As a sanity check, `<NEW_VERSION>` must match the version number defined by the provided bytecode, so upgrading to the wrong version can be prevented. `<MIGRATION_DATA>` is the json encoded data that will be passed to the contract's `migrate` function. If the flag is not provided, the default value `()` will be used, meaning that the migration data is of type `void`. The easiest way to generate the json data for complex types is to instantiate the rust type the contract expects and then use `serde_json::to_string` to convert it to json.

## Generate bindings

Generate TypeScript bindings for the contract

```bash
node stellar/generate-bindings.js --wasm-path /path/to/optimized.wasm --contract-id [contract-address] --output-dir ./stellar/bindings/[contract-name]
```

## Contract Interaction

Soroban contracts can be interacted directly via the CLI as well. See the help text for individual contract cmds as follows.

```bash
stellar contract invoke --network testnet --id [contract-address] --source-account wallet -- --help
```

### Gateway

To get help on the gateway commands, run:

```bash
node stellar/gateway.js --help
```

#### Call contract

```bash
node stellar/gateway.js call-contract [destination-chain] [dstination-address] [payload]

# Example
node stellar/gateway.js call-contract avalanche 0x4F4495243837681061C4743b74B3eEdf548D56A5 0x1234
```

#### Submit multisig prover proof

Submit a proof constructed on Amplifier to the Stellar gateway contract.

```bash
node stellar/gateway.js submit-proof [multisig-session-id]
```

#### Approve messages

A message approval can be submitted to the gateway contract for a test deployment where the wallet is the signer on the gateway. Setting `[destination-address]` to `wallet` will use the wallet address as the destination.

```bash
node stellar/gateway.js approve [source-chain] [message-id] [source-address] [destination-address] [payload]
```

#### Validate messages

An approved message can be validated by the gateway contract for a test deployment as follows:

```bash
node stellar/gateway.js validate-message [source-chain] [message-id] [source-address] [payload]
```

#### Rotate signers

A signer rotation can be submitted to the gateway contract. Use `--current-nonce` to override the default current nonce set for subsequent rotations. Skip `--signers` to rotate to the Amplifier verifier set registered in the prover contract.

```bash
node stellar/gateway.js rotate --new-nonce test --signers wallet
node stellar/gateway.js rotate --new-nonce test2 --current-nonce test --signers wallet
```

#### Execute

```bash
node stellar/gateway.js execute [source-chain] [message-id] [source-address] [destination-address] [payload]
```

### Interchain Token Service

_Note_: Stellar ITS runs only in Hub mode. P2P connections are not supported. Therefore, rather than setting trusted ITS addresses, we set trusted chains (chains which are also registered with ITS Hub). The ITS Hub chain (axelar) itself is not a valid source/destination for direct ITS messages and so shouldn't be set as a trusted chain. All ITS messages must be sent to and received from the ITS Hub.

#### Set Trusted Chain

```bash
node stellar/its.js set-trusted-chain [chain-name]
```

#### Remove Trusted Address

```bash
node stellar/its.js remove-trusted-chain [chain-name]
```

#### Deploy Interchain Token

```bash
node stellar/its.js deploy-interchain-token [name] [symbol] [decimal] [salt] [initial-supply]
```

#### Deploy Remote Interchain Token

```bash
node stellar/its.js deploy-remote-interchain-token [salt] [destination-chain] --gas-token-address [address] --gas-amount [amount]
```

#### Register Canonical Token

```bash
node stellar/its.js register-canonical-token [token-address]
```

#### Deploy Remote Canonical Token

```bash
node stellar/its.js deploy-remote-canonical-token [token-address] [destination-chain] --gas-token-address [address] --gas-amount [amount]
```

#### Interchain Transfer

```bash
node stellar/its.js interchain-transfer [token-id] [destination-chain] [destination-address] [amount] --data [data] --gas-token-address [address] --gas-amount [amount]
```

#### Encode stellar recipient address to bytes

```bash
node stellar/its.js encode-recipient 'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF'
```

#### Execute

```bash
node stellar/its.js execute [source-chain] [message-id] [source-address] [payload]
```

## TTL extension and state archival recovery

All Soroban storage entries, including contract instances, have a 'time to live' (`ttl`) after which entries will be archived and no longer accessible until restored. The following commands can be used to extend `ttl` or restore archived contract instances.

Get the ttl of a contract instance:

```bash
node stellar/contract.js get-ttl [contract-name]
```

Extend the ttl of a contract instance:

```bash
node stellar/contract.js extend-instance [contract-name]

# Defaults to maximum extension amount. To specify the number of ledgers to extend:
node stellar/contract.js extend-instance [contract-name] --extend-by [ledgers]
```

Restore an archived contract instance

```bash
node stellar/contract.js restore-instance [contract-name]
```

### Example

#### GMP - Send Command (Outgoing)

```bash
node stellar/gmp.js send [destination-chain] [destination-address] [payload] --gas-token-address [address] --gas-amount [amount]

# Example
node stellar/gmp.js send avalanche 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```

#### GMP - Execute Command (Incoming)

Note that approving the incoming message from the gateway needs to be done to execute it.

```bash
node stellar/gmp.js [source-chain] [message-id] [source-address] [payload]

# Example
node stellar/gmp.js execute avalanche '0x0bcbbfc9b006db6958f3fce75f11fdc306b45e8e43396211f414f40d2d6db7c5-0' 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```
