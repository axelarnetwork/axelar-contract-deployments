# Stellar deployments

## Installation

Install `libsodium` for faster signing operations optionally. Install npm dependencies afterwards.

```sh
brew install libsodium

npm ci
```

Install Soroban CLI

```bash
cargo install --locked soroban-cli --features opt
```

Add Soroban network in the config

```bash
soroban network add testnet --rpc-url https://soroban-testnet.stellar.org:443 "Test SDF Network ; September 2015" --global
```

Create a new Stellar keypair

```bash
soroban keys generate wallet --network testnet

# Address
soroban keys address wallet

# Get private key
soroban keys show wallet
```

Set `PRIVATE_KEY` in `.env` to the above value.

## Deployments

Setup

1. Checkout the axelar-cgp-soroban repo.
2. Compile the Soroban wasm contracts
```bash
cargo wasm --release
```
3. Optimize the contracts
```bash
./optimize.sh
```

### Gateway

Deploy the auth contract

```bash
node stellar/deploy-contract.js --contractName axelar_auth_verifiers --wasmPath ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_auth_verifier.optimized.wasm
```

Deploy the gateway contract and initialize it with the auth contract
```bash
node stellar/deploy-contract.js --contractName axelar_gateway --wasmPath ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_gateway.optimized.wasm --initialize
```

Initialize the auth contract. TODO: run initialize during the first step itself once initialize works correctly.
```bash
node stellar/deploy-contract.js --contractName axelar_auth_verifiers --wasmPath ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_auth_verifier.optimized.wasm --initialize --address [auth contract address]
```

### Operators

```bash
node stellar/deploy-contract.js --contractName axelar_operators --wasmPath ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_operators.optimized.wasm --initialize
```

## Generate bindings

Generate TypeScript bindings for the contract

```bash
node stellar/generate-bindings.js --wasmPath /path/to/optimized.wasm --contractId [contract address] --outputDir ./stellar/bindings/[contract name]
```

## Contract Interaction

Soroban contracts can be interacted directly via the CLI as well. See the help text for individual contract cmds as follows.

```bash
soroban contract invoke --network testnet --id [contract address] --source-account wallet -- --help
```
