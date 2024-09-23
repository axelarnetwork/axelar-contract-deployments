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

Testnet funds can be obtained via a [faucet](https://ftl.ai/), and transferred to your wallet.

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
node stellar/deploy-contract.js --contractName axelar_auth_verifier --wasmPath ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_auth_verifier.optimized.wasm --initialize
```

Deploy the gateway contract

```bash
node stellar/deploy-contract.js --contractName axelar_gateway --wasmPath ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_gateway.optimized.wasm --initialize
```

Provide `--estimateCost` to show the gas costs for the initialize transaction instead of executing it.

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
soroban contract invoke --network testnet --id [contract-address] --source-account wallet -- --help
```

### Gateway

#### Call contract

`node stellar/gateway.js call-contract ethereum 0x4F4495243837681061C4743b74B3eEdf548D56A5 0x1234`

#### Approve messages

A message approval can be submitted to the gateway contract. Replace `wallet` with another Stellar address if needed.
`node stellar/gateway.js approve ethereum id 0x1234 wallet 0x5678`

#### Rotate signers

A signer rotation can be submitted to the gateway contract. Use `--currentNonce` to override the default current nonce set for subsequent rotations. Skip `--signers` to rotate to the Amplifier verifier set registered in the prover contract.

```bash
node node stellar/gateway.js rotate --newNonce test --signers wallet

node node stellar/gateway.js rotate --newNonce test2 --currentNonce test --signers wallet
```
