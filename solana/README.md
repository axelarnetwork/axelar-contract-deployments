# Solana deployments

## Instalation

1. Ensure you have Rust installed. If you don't:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Install Solana CLI:

```sh
sh -c "$(curl -sSfL https://release.anza.xyz/v2.2.14/install)"
```

3. Install `solana-verify`, for verifiable builds:

```sh
cargo install solana-verify
```

4. Create a new  Solana keypair

```sh
# Set default cluster
solana config set --url devnet

# Generate a new keypair
solana-keygen new 

# Address
solana address

# Generate a new keypair and overwrite the previously generated one
solana-keygen new --force

```

5. Devnet funds can be obtained via [this link](https://faucet.solana.com/) or using the Solana CLI:

```sh
solana airdrop 2
```

## Deployments

Setup

1. Clone the solana-axelar repo.
2. Compile the Solana programs.

> [!IMPORTANT]
> For the initial deployment of Solana programs to any of the clusters (devnet, testnet, and mainnet-beta), the program keypairs are required. The pubkey is the program ID and is hardcoded in the program using the `declare_id` macro. In case a new set of keypairs is required, a new release of the crates needs to happen afterwards (due to the id being hardcoded). Updating the ids can be done within the `solana-axelar` directory by invoking:
> ```sh
> cargo xtask update-ids
> ```
> The keypair files should be stored securely as they're needed for the initial deployment on other clusters as well.

> [!NOTE]
> Initial deployment of Solana programs doesn't support offline signing, the process needs to be done online. When deploying, an `upgrade-authority` can be set, which will later be able to perform program upgrades — upgrades support offline signing.

In order to get verifiable builds, we use `solana-verify` tool. For more information on how to use the tool - including when multisig is used (which is expected as upgrade authority for mainnet deployments) - visit the [Solana guide for verifiable builds](https://solana.com/developers/guides/advanced/verified-builds).

Set the `BASE_IMAGE` variable:

```sh
export BASE_IMAGE="solanafoundation/solana-verifiable-build@sha256:979b09eef544de4502a92e28a724a8498a08e2fe506e8905b642e613760403d3"
```

```sh
# Go to the solana directory within the cloned repo
pushd solana-axelar

# Compile the Solana programs
solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_gas_service
solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_gateway
solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_governance
solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_its
solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_multicall

# Go back
popd
```

3. Declare environment variables:

```sh
GATEWAY_PROGRAM_KEYPAIR_PATH=<path/to/gateway_program_keypair.json>
GATEWAY_PROGRAM_PATH="solana-axelar/target/deploy/axelar_solana_gateway.so"

GAS_SERVICE_PROGRAM_KEYPAIR_PATH=<path/to/gas_service_program_keypair.json>
GAS_SERVICE_PROGRAM_PATH="solana-axelar/target/deploy/axelar_solana_gas_service.so"

GOVERNANCE_PROGRAM_KEYPAIR_PATH=<path/to/governance_program_keypair.json>
GOVERNANCE_PROGRAM_PATH="solana-axelar/target/deploy/axelar_solana_governance.so"

MULTICALL_PROGRAM_KEYPAIR_PATH=<path/to/multicall_program_keypair.json>
MULTICALL_PROGRAM_PATH="solana-axelar/target/deploy/axelar_solana_multicall.so"

ITS_PROGRAM_KEYPAIR_PATH=<path/to/its_program_keypair.json>
ITS_PROGRAM_PATH="solana-axelar/target/deploy/axelar_solana_its.so"

UPGRADE_AUTHORITY_KEYPAIR_PATH=<path/to/upgrade_authority_keypair.json>
```
```bash
set -a
source .env
set +a
```

### Gateway

Deploy and verify the gateway program. If `--authority` is omitted, the current Solana CLI keypair is set as upgrade-authority.

```sh
solana program-v4 deploy --program-keypair $GATEWAY_PROGRAM_KEYPAIR_PATH --authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $GATEWAY_PROGRAM_PATH


solana-verify verify-from-repo --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $GATEWAY_PROGRAM_KEYPAIR_PATH) https://github.com/eigerco/solana-axelar --library-name axelar_solana_gateway
```

### Gas Service

Deploy and verify the gas service program (note that verification will only work on mainnet)

```sh
solana program-v4 deploy --program-keypair $GAS_SERVICE_PROGRAM_KEYPAIR_PATH --authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $GAS_SERVICE_PROGRAM_PATH

solana-verify verify-from-repo --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $GAS_SERVICE_PROGRAM_KEYPAIR_PATH) https://github.com/eigerco/solana-axelar --library-name axelar_solana_gas_service
```

### Governance

Deploy and verify the governance program (note that verification will only work on mainnet)

```sh
solana program-v4 deploy --program-keypair $GOVERNANCE_PROGRAM_KEYPAIR_PATH --authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $GOVERNANCE_PROGRAM_PATH

solana-verify verify-from-repo --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $GOVERNANCE_PROGRAM_KEYPAIR_PATH) https://github.com/eigerco/solana-axelar --library-name axelar_solana_governance
```

### Multicall

Deploy and verify the multicall program (note that verification will only work on mainnet)

```sh
solana program-v4 deploy --program-keypair $MULTICALL_PROGRAM_KEYPAIR_PATH --authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $MULTICALL_PROGRAM_PATH

solana-verify verify-from-repo --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $MULTICALL_PROGRAM_KEYPAIR_PATH) https://github.com/eigerco/solana-axelar --library-name axelar_solana_multicall
```

### Interchain Token Service

Deploy and verify the ITS program (note that verification will only work on mainnet)

```sh
solana program-v4 deploy --program-keypair $ITS_PROGRAM_KEYPAIR_PATH --authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $ITS_PROGRAM_PATH

solana-verify verify-from-repo --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $ITS_PROGRAM_KEYPAIR_PATH) https://github.com/eigerco/solana-axelar --library-name axelar_solana_its
```

## Upgrades

To upgrade a program, a similar command is used as for the initial deployment, but with the `--program-id` option instead of `--program-keypair`. The upgrade should be performed by the authority assigned during the initial deployment.


```sh
solana program-v4 deploy --program-id <PROGRAM_ID_PUBKEY> <PATH_TO_PROGRAM_SO>
```

For upgrades with offline signing, recovery of failed deployments, and other information about Solana program deployment, please check the [official docs](https://solana.com/docs/programs/deploying).

---

## Contract Interaction

Solana contracts can be interacted with using the provided CLI. The CLI supports both direct execution and offline signing workflows.

### CLI Usage

The Solana Axelar CLI (`solana-axelar-cli`) provides several commands for interacting with the deployed contracts. The basic usage is:

```sh
./solana-axelar-cli <COMMAND> [OPTIONS] [SUBCOMMAND]
```

Main commands:

- `send`: Build and send a transaction to the Solana network
- `generate`: Generate an unsigned transaction for offline signing
- `sign`: Sign an unsigned transaction using a local keypair or Ledger
- `combine`: Combine multiple partial signatures into a single file
- `broadcast`: Broadcast a combined signed transaction to the Solana network
- `misc`: Miscellaneous utilities
- `query`: Commands used to query accounts or emitted events

`send`, `generate`, and `query` have associated subcommands for specific contract interactions.

### Network Configuration

There are a few different ways you can specify the Solana network to connect to:

By exporting the `CLUSTER` variable in your shell:

```sh
export CLUSTER=<URL_OR_MONIKER>
```

By creating a `.env` file in the root of the project with the `CLUSTER=<URL_OR_MONIKER>` entry or, on every command:

```sh
./solana-axelar-cli --url <URL_OR_MONIKER> <COMMAND> [OPTIONS]
```

The value can be a full RPC URL or a moniker:
- `mainnet-beta`: Solana mainnet
- `testnet`: Solana testnet
- `devnet`: Solana devnet
- `localhost`: Local Solana validator

If none of these options are provided, the value set in the default Solana CLI config file will be used (if available).

### Wallet configuration

Similarly to the network configuration, you can specify the wallet to use in a few different ways:

By exporting the `PRIVATE_KEY` variable in your shell:

```sh
export PRIVATE_KEY=<PATH_TO_KEYPAIR>
```

By creating a `.env` file in the root of the project with the `PRIVATE_KEY=<PATH_TO_KEYPAIR>` entry or, on every use of the `send` command:

```sh
./solana-axelar-cli send --fee-payer <PATH_TO_KEYPAIR> [OPTIONS]
```

The value can be a path to a solana keypair JSON file (generated with `solana-keypair new`) or the USB path to a Ledger device (e.g.: usb://ledger).

### Gateway

To get help on gateway commands, run:

```sh
./solana-axelar-cli send --help
./solana-axelar-cli send gateway --help
```

#### Initialize Gateway

```sh
./solana-axelar-cli --url --env devnet-amplifier send gateway init \
  --previous-signers-retention 15 \
  --minimum-rotation-delay 15 \
  --operator E9yYxCfQmP1UFP8LHLqRQ68LaYmFKD56Gm568tKwtWjA
```

#### Call Contract

Send a message to another chain:

```sh
./solana-axelar-cli send gateway call-contract \
  --destination-chain <DESTINATION_CHAIN_NAME> \
  --destination-address <DESTINATION_ADDRESS> \
  --payload <MESSAGE_PAYLOAD_HEX>
```

#### Submit Proof

Submit a proof constructed on Amplifier to the Solana gateway contract:

```sh
./solana-axelar-cli send gateway submit-proof \
  --multisig-session-id <MULTISIG_SESSION_ID>
```

#### Execute Message

Execute a cross-chain message that was approved on the Solana gateway:

```sh
./solana-axelar-cli send gateway execute \
  --source-chain <SOURCE_CHAIN_NAME> \
  --message-id <MESSAGE_ID> \
  --source-address <SOURCE_ADDRESS> \
  --destination-address <DESTINATION_PUBKEY> \
  --payload <MESSAGE_PAYLOAD_HEX>
```

#### Rotate Signers

```sh
./solana-axelar-cli send gateway rotate \
  --signer <SIGNER_PRIVATE_KEY_HEX> \
  --nonce <CURRENT_NONCE> \
  --new-nonce <NEW_NONCE>
```

#### Transfer Operatorship

```sh
# Transfer Gateway operatorship
./solana-axelar-cli send gateway transfer-operatorship \
  --authority <CURRENT_AUTHORITY_PUBKEY> \
  --new-operator <NEW_OPERATOR_PUBKEY>
```

### Interchain Token Service (ITS)

To get help on ITS commands, run:

```sh
./solana-axelar-cli send its --help
```

#### Initialize ITS

```sh
./solana-axelar-cli send its init --operator <OPERATOR_PUBKEY>
```

#### Set/Remove Trusted Chain

```sh
# Add a trusted chain
./solana-axelar-cli send its set-trusted-chain <CHAIN_NAME>

# Remove a trusted chain
./solana-axelar-cli send its remove-trusted-chain <CHAIN_NAME>
```

#### Deploy Interchain Token

```sh
./solana-axelar-cli send its deploy-interchain-token \
  --salt <SALT_STRING> \
  --name <TOKEN_NAME> \
  --symbol <TOKEN_SYMBOL> \
  --decimals <DECIMALS> \
  --initial-supply <INITIAL_SUPPLY>
```

#### Deploy Remote Interchain Token

```sh
./solana-axelar-cli send its deploy-remote-interchain-token \
  --salt <SALT_STRING> \
  --destination-chain <DESTINATION_CHAIN_NAME> \
  --gas-value <GAS_VALUE>
```

#### Register Canonical Token

```sh
./solana-axelar-cli send its register-canonical-interchain-token \
  --mint <MINT_ADDRESS> \
  --token-program <TOKEN_PROGRAM>
```

#### Deploy Remote Canonical Token

```sh
./solana-axelar-cli send its deploy-remote-canonical-interchain-token \
  --mint <MINT_ADDRESS> \
  --destination-chain <DESTINATION_CHAIN_NAME> \
  --gas-value <GAS_VALUE>
```

#### Interchain Transfer

```sh
./solana-axelar-cli send its interchain-transfer \
  --source-account <SOURCE_ACCOUNT_ADDRESS> \
  --token-id <TOKEN_ID_HEX> \
  --destination-chain <DESTINATION_CHAIN_NAME> \
  --destination-address <DESTINATION_ADDRESS> \
  --amount <AMOUNT> \
  --mint <MINT_ADDRESS> \
  --token-program <TOKEN_PROGRAM> \
  --gas-value <GAS_VALUE>
```

#### Call Contract With Interchain Token

```sh
./solana-axelar-cli send its call-contract-with-interchain-token \
  --source-account <SOURCE_ACCOUNT_ADDRESS> \
  --token-id <TOKEN_ID_HEX> \
  --destination-chain <DESTINATION_CHAIN_NAME> \
  --destination-address <DESTINATION_ADDRESS> \
  --amount <AMOUNT> \
  --mint <MINT_ADDRESS> \
  --data <PAYLOAD_DATA> \
  --token-program <TOKEN_PROGRAM> \
  --gas-value <GAS_VALUE>
```

#### Set Flow Limit

```sh
./solana-axelar-cli send its set-flow-limit \
  --token-id <TOKEN_ID_HEX> \
  --flow-limit <FLOW_LIMIT>
```

#### Pausable Functionality

```sh
# Check if ITS is paused
./solana-axelar-cli send its paused

# Pause ITS
./solana-axelar-cli send its pause

# Unpause ITS
./solana-axelar-cli send its unpause
```

#### TokenManager Operations

```sh
# Set flow limit on a token manager
./solana-axelar-cli send its token-manager set-flow-limit \
  --token-id <TOKEN_ID_HEX> \
  --flow-limit <FLOW_LIMIT>

# Add flow limiter to a token manager
./solana-axelar-cli send its token-manager add-flow-limiter \
  --token-id <TOKEN_ID_HEX> \
  --flow-limiter <FLOW_LIMITER_PUBKEY>
```

#### InterchainToken Operations

```sh
# Mint interchain tokens
./solana-axelar-cli send its interchain-token mint \
  --token-id <TOKEN_ID_HEX> \
  --mint <MINT_ADDRESS> \
  --to <DESTINATION_ACCOUNT> \
  --token-program <TOKEN_PROGRAM> \
  --amount <AMOUNT>
```

#### Transfer Operatorship

```sh
# Transfer ITS operatorship
./solana-axelar-cli send its transfer-operatorship --to <NEW_OPERATOR_PUBKEY>
```
## Governance

For governance-related commands:

```sh
./solana-axelar-cli send governance --help
```

## Gas Service

For gas service commands:

```sh
./solana-axelar-cli send gas-service --help
```

### Offline Signing Workflow

For security-critical operations or when using hardware wallets, you can use the offline signing workflow:

#### Creating a Durable Nonce Account

Durable nonces are necessary for offline signing to ensure transactions remain valid across block hashes. To create and manage a durable nonce account:

```sh
# Create a nonce account (requires SOL for rent exemption)
solana create-nonce-account <NONCE_ACCOUNT_KEYPAIR_PATH> <AMOUNT_SOL> --nonce-authority <AUTHORITY_PUBKEY>
```

#### 1. Generate the unsigned transaction

```sh
./solana-axelar-cli generate \
  --fee-payer <FEE_PAYER_PUBKEY> \
  --nonce-account <NONCE_ACCOUNT_PUBKEY> \
  --nonce-authority <NONCE_AUTHORITY_PUBKEY> \
  gateway call-contract \
  --destination-chain <DESTINATION_CHAIN_NAME> \
  --destination-address <DESTINATION_ADDRESS> \
  --payload <MESSAGE_PAYLOAD_HEX>
```

This will generate a file like `./output/gateway-init.unsigned.json` in the default output directory. You can specify a custom output directory with `--output-dir /path/to/directory`.

#### 2. Sign the transaction (on each signing device)

```sh
./solana-axelar-cli sign <PATH_TO_SIGNER_KEYPAIR> <PATH_TO_UNSIGNED_TX_JSON> 
```
`PATH_TO_SIGNER_KEYPAIR` can be a local keypair file or a Ledger device.
```sh

This will generate signature files like `./output/gateway-init.5hW1cNgX6N8RhvHHiX6nAnKbZftG1K3ckNBuJdRSPFPK.partial.sig` where the signer's full public key is included in the filename.

#### 3. Combine all signatures

```sh
./solana-axelar-cli combine <PATH_TO_UNSIGNED_TX_JSON> <PATH_TO_SIGNATURE_1> <PATH_TO_SIGNATURE_2> [...]
```

This will generate a file like `./output/gateway-init.signed.json`.

#### 4. Broadcast the transaction

```sh
./solana-axelar-cli broadcast <PATH_TO_SIGNED_TX_JSON>
```
