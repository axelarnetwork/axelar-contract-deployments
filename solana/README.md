# Solana deployments

## Instalation

Install Solana CLI

```sh
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
```

For more info on how to install the Solana tooling, check the official documentation [here](https://solana.com/docs/intro/installation).

Create a new  Solana keypair

```sh
# Set default cluster
solana config set --url testnet 

# Generate a new keypair
solana-keygen new 

# Address
solana address

# Generate a new keypair and overwrite the previously generated one
solana-keygen new --force

```

Testnet funds can be obtained via [this link](https://faucet.solana.com/) or using the Solana CLI:

```sh
solana airdrop 2
```

## Deployments

Setup

1. Clone the solana-axelar repo.
2. Compile the Solana programs.

> [!IMPORTANT]
> For the initial deployment of Solana programs to any of the clusters (devnet, testnet, and mainnet-beta), the program keypairs are required. The pubkey is the program ID and is hardcoded in the program using the `declare_id` macro. In case a new set of keypairs is required, a new release of the crates needs to happen afterwards (due to the id being hardcoded). Updating the ids can be done within the `solana-axelar/solana` directory by invoking:
> ```sh
> cargo xtask update-ids
> ```
> The keypair files should be stored securely as they're needed for the initial deployment on other clusters as well.

> [!NOTE]
> Initial deployment of Solana programs doesn't support offline signing, the process needs to be done online. When deploying, an `upgrade-authority` can be set, which will later be able to perform program upgrades — upgrades support offline signing.

```sh
# Go to the solana directory within the cloned repo
pushd solana-axelar/solana/

# Compile the Solana programs
cargo xtask build

# Go back
popd
```

### Gateway

Deploy the gateway program. If `--authority` is omitted, the current Solana CLI keypair is set as upgrade-authority.

```sh
solana program-v4 deploy --program-keypair path/to/gateway-program-keypair.json --authority <base58 pubkey> solana-axelar/solana/target/sbf-solana-solana/release/axelar_solana_gateway.so
```

### Gas Service

Deploy the gas service program

```sh
solana program-v4 deploy --program-keypair path/to/gas-service-program-keypair.json --authority <base58 pubkey> solana-axelar/solana/target/sbf-solana-solana/release/axelar_solana_gas_service.so
```

### Interchain Token Service

Deploy the ITS program

```sh
solana program-v4 deploy --program-keypair path/to/its-program-keypair.json --authority <base58 pubkey> solana-axelar/solana/target/sbf-solana-solana/release/axelar_solana_its.so
```

### Governance

Deploy the governance program

```sh
solana program-v4 deploy --program-keypair path/to/governance-program-keypair.json --authority <base58 pubkey> solana-axelar/solana/target/sbf-solana-solana/release/axelar_solana_governance.so
```

### Multicall

Deploy the multicall program

```sh
solana program-v4 deploy --program-keypair path/to/multicall-program-keypair.json --authority <base58 pubkey> solana-axelar/solana/target/sbf-solana-solana/release/axelar_solana_multicall.so
```

## Upgrades

To upgrade a program, a similar command is used as for the initial deployment, but with the `--program-id` option instead of `--program-keypair`. The upgrade should be performed by the authority assigned during the initial deployment.


```sh
solana program-v4 deploy --program-id <base58 pubkey> <path/to/program.so>
```

For upgrades with offline signing, recovery of failed deployments, and other information about Solana program deployment, please check the [official docs](https://solana.com/docs/programs/deploying).

---

## Contract Interaction

Solana contracts can be interacted with using the provided CLI. The CLI supports both direct execution and offline signing workflows.

### CLI Usage

The Solana Axelar CLI (`solana-axelar-cli`) provides several commands for interacting with the deployed contracts. The basic usage is:

```sh
solana/solana-axelar-cli <COMMAND> [OPTIONS] [SUBCOMMAND]
```

Main commands:

- `send`: Build and send a transaction to the Solana network
- `generate`: Generate an unsigned transaction for offline signing
- `sign`: Sign an unsigned transaction using a local keypair or Ledger
- `combine`: Combine multiple partial signatures into a single file
- `broadcast`: Broadcast a combined signed transaction to the Solana network
- `misc`: Miscellaneous utilities

`send` and `generate` have associated subcommands for specific contract interactions.

### Network Configuration

Specify the Solana network to connect to:

```sh
export URL_OR_MONIKER=<value>
```

or, on every command:

```sh
solana/solana-axelar-cli --url <URL_OR_MONIKER> <COMMAND> [OPTIONS]
```

The URL can be a full RPC URL or a moniker:
- `mainnet-beta`: Solana mainnet
- `testnet`: Solana testnet
- `devnet`: Solana devnet
- `localhost`: Local Solana validator

### Gateway

To get help on gateway commands, run:

```sh
solana/solana-axelar-cli send --help
solana/solana-axelar-cli send gateway --help
```

#### Initialize Gateway

```sh
solana/solana-axelar-cli send gateway init \
  --previous-signers-retention 3 \
  --minimum-rotation-delay 86400 \
  --operator <PUBKEY>
```

#### Call Contract

Send a message to another chain:

```sh
solana/solana-axelar-cli send gateway call-contract \
  --destination-chain avalanche \
  --destination-address 0x4F4495243837681061C4743b74B3eEdf548D56A5 \
  --payload 0x1234
```

#### Submit Proof

Submit a proof constructed on Amplifier to the Solana gateway contract:

```sh
solana/solana-axelar-cli send gateway submit-proof \
  --multisig-session-id 123456
```

#### Execute Message

Execute a cross-chain message that was approved on the Solana gateway:

```sh
solana/solana-axelar-cli send gateway execute \
  --source-chain avalanche \
  --message-id '0x0bcbbfc9b006db6958f3fce75f11fdc306b45e8e43396211f414f40d2d6db7c5-0' \
  --source-address 0xba76c6980428A0b10CFC5d8ccb61949677A61233 \
  --destination-address <PUBKEY> \
  --payload 0x1234
```

#### Rotate Signers

```sh
solana/solana-axelar-cli send gateway rotate \
  --signer <PRIVATE_KEY_HEX> \
  --nonce 123 \
  --new-nonce 456
```

#### Transfer Operatorship

```sh
# Transfer Gateway operatorship
solana/solana-axelar-cli send gateway transfer-operatorship \
  --authority <CURRENT_AUTHORITY_PUBKEY> \
  --new-operator <NEW_OPERATOR_PUBKEY>
```

### Interchain Token Service (ITS)

To get help on ITS commands, run:

```sh
solana/solana-axelar-cli send its --help
```

#### Initialize ITS

```sh
solana/solana-axelar-cli send its init --operator <PUBKEY>
```

#### Set/Remove Trusted Chain

```sh
# Add a trusted chain
solana/solana-axelar-cli send its set-trusted-chain --chain-name avalanche

# Remove a trusted chain
solana/solana-axelar-cli send its remove-trusted-chain --chain-name avalanche
```

#### Deploy Interchain Token

```sh
solana/solana-axelar-cli send its deploy-interchain-token \
  --salt <SALT_STRING> \
  --name "My Token" \
  --symbol "MTK" \
  --decimals 8 \
  --initial-supply 1000000000
```

#### Deploy Remote Interchain Token

```sh
solana/solana-axelar-cli send its deploy-remote-interchain-token \
  --salt <SALT_STRING> \
  --destination-chain avalanche \
  --gas-value 500000
```

#### Register Canonical Token

```sh
solana/solana-axelar-cli send its register-canonical-interchain-token \
  --mint <MINT_ADDRESS> \
  --token-program spl_token
```

#### Deploy Remote Canonical Token

```sh
solana/solana-axelar-cli send its deploy-remote-canonical-interchain-token \
  --mint <MINT_ADDRESS> \
  --destination-chain avalanche \
  --gas-value 500000
```

#### Interchain Transfer

```sh
solana/solana-axelar-cli send its interchain-transfer \
  --source-account <ACCOUNT_ADDRESS> \
  --token-id <TOKEN_ID_HEX> \
  --destination-chain avalanche \
  --destination-address 0x4F4495243837681061C4743b74B3eEdf548D56A5 \
  --amount 1000000 \
  --mint <MINT_ADDRESS> \
  --token-program spl_token \
  --gas-value 500000
```

#### Call Contract With Interchain Token

```sh
solana/solana-axelar-cli send its call-contract-with-interchain-token \
  --source-account <ACCOUNT_ADDRESS> \
  --token-id <TOKEN_ID_HEX> \
  --destination-chain avalanche \
  --destination-address 0x4F4495243837681061C4743b74B3eEdf548D56A5 \
  --amount 1000000 \
  --mint <MINT_ADDRESS> \
  --data 0x1234 \
  --token-program spl_token \
  --gas-value 500000
```

#### Set Flow Limit

```sh
solana/solana-axelar-cli send its set-flow-limit \
  --token-id <TOKEN_ID_HEX> \
  --flow-limit 1000000
```

#### Pausable Functionality

```sh
# Check if ITS is paused
solana/solana-axelar-cli send its paused

# Pause ITS
solana/solana-axelar-cli send its pause

# Unpause ITS
solana/solana-axelar-cli send its unpause
```

#### TokenManager Operations

```sh
# Set flow limit on a token manager
solana/solana-axelar-cli send its token-manager set-flow-limit \
  --token-id <TOKEN_ID_HEX> \
  --flow-limit 1000000

# Add flow limiter to a token manager
solana/solana-axelar-cli send its token-manager add-flow-limiter \
  --token-id <TOKEN_ID_HEX> \
  --flow-limiter <PUBKEY>
```

#### InterchainToken Operations

```sh
# Mint interchain tokens
solana/solana-axelar-cli send its interchain-token mint \
  --token-id <TOKEN_ID_HEX> \
  --mint <MINT_ADDRESS> \
  --to <DESTINATION_ACCOUNT> \
  --token-program spl_token \
  --amount 1000000
```

#### Transfer Operatorship

```sh
# Transfer ITS operatorship
solana/solana-axelar-cli send its transfer-operatorship --to <NEW_OPERATOR_PUBKEY>
```
## Governance

For governance-related commands:

```sh
solana/solana-axelar-cli send governance --help
```

## Gas Service

For gas service commands:

```sh
solana/solana-axelar-cli send gas-service --help
```

### Offline Signing Workflow

For security-critical operations or when using hardware wallets, you can use the offline signing workflow:

#### Creating a Durable Nonce Account

Durable nonces are necessary for offline signing to ensure transactions remain valid across block hashes. To create and manage a durable nonce account:

```sh
# Create a nonce account (requires SOL for rent exemption)
solana create-nonce-account <KEYPAIR_PATH> <AMOUNT_SOL> --nonce-authority <AUTHORITY_PUBKEY>
```

#### 1. Generate the unsigned transaction

```sh
solana/solana-axelar-cli generate \
  --fee-payer <FEE_PAYER_PUBKEY> \
  --nonce-account <NONCE_ACCOUNT_PUBKEY> \
  --nonce-authority <NONCE_AUTHORITY_PUBKEY> \
  gateway call-contract \
  --destination-chain avalanche \
  --destination-address 0x4F4495243837681061C4743b74B3eEdf548D56A5 \
  --payload 0x1234
```

This will generate a file like `./output/gateway_init.unsigned.json` in the default output directory. You can specify a custom output directory with `--output-dir /path/to/directory`.

#### 2. Sign the transaction (on each signing device)

```sh
solana/solana-axelar-cli sign \
  ./output/gateway_init.unsigned.json \
  --signer /path/to/keypair.json
```

For Ledger:

```sh
solana/solana-axelar-cli sign \
  ./output/gateway_init.unsigned.json \
  --signer usb://ledger
```

This will generate signature files like `./output/gateway_init.5hW1cNgX6N8RhvHHiX6nAnKbZftG1K3ckNBuJdRSPFPK.partial.sig` where the signer's full public key is included in the filename for uniqueness.

#### 3. Combine all signatures

```sh
solana/solana-axelar-cli combine \
  --unsigned-tx-path ./output/gateway_init.unsigned.json \
  --signatures ./output/gateway_init.5hW1cNgX6N8RhvHHiX6nAnKbZftG1K3ckNBuJdRSPFPK.partial.sig ./output/gateway_init.DL6NBsMvnEMbUJ5XHeLMyfGpmEukV2i7ZVukGCfxWvP5.partial.sig
```

This will generate a file like `./output/gateway_init.signed.json`.

#### 4. Broadcast the transaction

```sh
solana/solana-axelar-cli broadcast \
  ./output/gateway_init.signed.json
```
