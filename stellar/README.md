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

Testnet funds can be obtained via a [faucet](https://ftl.ai/), and transferred to your wallet.

## Deployments

Setup

1. Checkout the axelar-cgp-soroban repo.
2. Compile the Soroban wasm contracts

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
node stellar/generate-bindings.js --wasmPath /path/to/optimized.wasm --contractId [contract-address] --outputDir ./stellar/bindings/[contract-name]
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

### Submit multisig prover proof

Submit a proof constructed on Amplifier to the Stellar gateway contract.

```bash
node stellar/gateway.js submit-proof [multisig-session-id]
```

### Approve messages

A message approval can be submitted to the gateway contract for a test deployment where the wallet is the signer on the gateway. Setting `[destination_address]` to `wallet` will use the wallet address as the destination.

```bash
node stellar/gateway.js approve [source-chain] [message-id] [source-address] [destination-address] [payload]
```

### Validate messages

An approved message can be validated by the gateway contract for a test deployment as follows:

```bash
node stellar/gateway.js validate-message [source-chain] [message-id] [source-address] [payload]
```

### Rotate signers

A signer rotation can be submitted to the gateway contract. Use `--currentNonce` to override the default current nonce set for subsequent rotations. Skip `--signers` to rotate to the Amplifier verifier set registered in the prover contract.

```bash
node stellar/gateway.js rotate --newNonce test --signers wallet
node stellar/gateway.js rotate --newNonce test2 --currentNonce test --signers wallet
```
