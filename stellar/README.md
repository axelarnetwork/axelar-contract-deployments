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
node stellar/deploy-contract.js deploy axelar_gateway --chain-name <CHAIN_NAME> --wasm-path ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_gateway.optimized.wasm --initialize
```

Provide `--estimate-cost` to show the gas costs for the initialize transaction instead of executing it.

### Operators

```bash
node stellar/deploy-contract.js deploy axelar_operators --chain-name <CHAIN_NAME> --wasm-path ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_operators.optimized.wasm --initialize
```

### Gas Service

```bash
node stellar/deploy-contract.js deploy axelar_gas_service --chain-name <CHAIN_NAME> --wasm-path ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_gas_service.optimized.wasm --initialize
```

### Interchain Token Service

```bash
node stellar/deploy-contract.js deploy interchain_token_service --wasm-path ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/interchain_token_service.optimized.wasm --initialize
```

### Example

```bash
node stellar/deploy-contract.js deploy example --wasm-path ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/example.optimized.wasm --initialize
```

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

#### Upgrade Gateway

To upgrade the gateway, run the following command:

```bash
node stellar/deploy-contract.js upgrade axelar_gateway --wasm-path ../axelar-cgp-soroban/target/wasm32-unknown-unknown/release/axelar_gateway.optimized.wasm
```

### Interchain Token Service

#### Set Trusted Address

```bash
node stellar/its.js set-trusted-address [chain-name] [trusted-address]
```

#### Remove Trusted Address

```bash
node stellar/its.js remove-trusted-address [chain-name]
```

### Example

#### GMP - Send

```bash
node stellar/gmp.js send [destination-chain] [destination-address] [payload] [gas-token-address] [gas-fee-amount]

# Example
node stellar/gmp.js send avalanche 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234 CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC 1
```

#### GMP - Execute

```bash
node stellar/gmp.js [source-chain] [message-id] [source-address] [payload]

# Example
node stellar/gmp.js execute avalanche '0x0bcbbfc9b006db6958f3fce75f11fdc306b45e8e43396211f414f40d2d6db7c5-0' 0xba76c6980428A0b10CFC5d8ccb61949677A61233 0x1234
```
