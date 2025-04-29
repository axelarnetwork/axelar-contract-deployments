# Solana Key Management Tool

## Overview

This tool facilitates the generation, signing, and broadcasting of Solana transactions, with a strong emphasis on security workflows, particularly for offline and multisig scenarios. It is designed to align with best practices where transaction generation and signing are separated, especially for high-value mainnet operations.

Key features include:
*   **Offline Signing Support (Mainnet):** Generates a self-contained bundle (`.tar.gz`) containing the unsigned transaction and necessary tools (like this binary itself) for offline signing using a Ledger hardware wallet.
*   **Multisig Workflow:** Supports generating partial signatures from multiple signers and combining them into a single transaction.
*   **Durable Nonce Support:** Allows using Solana's durable nonces for transactions that might experience delays between creation and broadcasting (essential for offline/multisig).
*   **Environment Awareness:** Differentiates between Mainnet (requires Ledger, offline packaging) and Testnet/Devnet (allows local keypair files, no packaging needed).
*   **Clear Steps:** Follows a Generate -> Sign -> Combine -> Broadcast workflow via specific subcommands.

## Prerequisites

1.  **Rust & Cargo:** Needed to build the tool. ([https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install))
2.  **Solana Tool Suite (Optional but Recommended):** Useful for generating keypairs (`solana-keygen`) and creating/managing nonce accounts (`solana create-nonce-account`). ([https://docs.solana.com/cli/install](https://docs.solana.com/cli/install))
3.  **Keypairs:**
    *   For Testnet/Devnet: Keypair files (e.g., `~/.config/solana/id.json`).
    *   For Mainnet: A Ledger hardware wallet with the Solana app installed and the relevant account derived. You will need the *public key* of the account on the Ledger.
4.  **Nonce Account (Optional):** If using the durable nonce feature, you must have already created and funded a nonce account on the target network. You will need its public key and the public key of its authority.
5.  **Instruction Definition File:** A JSON file defining the core instructions for the transaction (see details below).

## Installation

1.  Clone the repository (if applicable) or navigate to the project directory.
2.  Build the release binary:
    ```bash
    cargo build --release
    ```
3.  The executable will be located at `target/release/solana_key_management_tool` (or your package name).
4.  (Optional) Copy the executable to a directory in your system's `PATH` for easier access, or call it directly using its full path.

## Core Workflow

The tool operates in distinct steps:

1.  **Generate (`generate`):**
    *   Takes transaction details (fee payer, instructions, nonce/blockhash info).
    *   Fetches necessary on-chain data (latest blockhash or nonce value).
    *   Creates an unsigned transaction file (`*.unsigned.solana.json`).
    *   **For Mainnet:** Packages the unsigned transaction and signing tools into an offline bundle (`*.solana.bundle.tar.gz`). This bundle is transferred securely to offline signing machines.
    *   **For Testnet/Devnet:** Only the unsigned transaction file is created.

2.  **Sign (`sign`):**
    *   Takes the unsigned transaction file.
    *   **For Mainnet:** Requires the signer's *public key* (Base58) on the Ledger (`--signer-key <PUBKEY>`). This step must be performed on a secure, offline machine using the unpacked bundle from step 1. It interacts with the connected Ledger device (currently simulated).
    *   **For Testnet/Devnet:** Requires the *path* to the signer's keypair file (`--signer-key /path/to/key.json`). Can be done on the same machine.
    *   Outputs a partial signature file (`*.sig.json`). This file needs to be securely transferred back from offline environments if applicable.

3.  **Combine (`combine`):**
    *   Takes the original unsigned transaction file and one or more partial signature files.
    *   Verifies that signatures from all required signers (fee payer, nonce authority, instruction signers) are present.
    *   Merges the signatures into a combined signed transaction file (`*.signed.solana.json`).

4.  **Broadcast (`broadcast`):**
    *   Takes the combined signed transaction file.
    *   Reconstructs the Solana transaction object.
    *   Submits the transaction to the target network via the specified RPC URL.
    *   Waits for confirmation and reports the transaction signature (ID).

## Command Reference

Replace `solana-kms-tool` with the actual path to your compiled binary (e.g., `./target/release/solana_key_management_tool`).

**Global Options (Apply to all commands):**

*   `-u, --rpc-url <URL>`: (Required) Solana JSON RPC endpoint URL.
*   `-n, --network <NETWORK>`: Target network (`mainnet-beta`, `testnet`, `devnet`). Default: `devnet`. Can be set via `SOLANA_NETWORK` env var.
*   `-o, --output-dir <PATH>`: Directory for output files. Default: `./output`.
*   `--keypair-dir <PATH>`: (Optional) Default directory for finding keypairs (used by some non-mainnet flows if full path isn't given). Can be set via `SOLANA_KEY_DIR` env var.

**Commands:**

### `generate`

Generates an unsigned transaction file and potentially an offline bundle.

```bash
solana-kms-tool generate [OPTIONS] --fee-payer <PUBKEY> --instructions <PATH> --output-name <BASENAME>
```

**Arguments:**

*   `--fee-payer <PUBKEY>`: (Required) Base58 public key of the fee payer.
*   `-i, --instructions <PATH>`: (Required) Path to a JSON file containing the list of instructions. See "Instruction File Format" below.
*   `--output-name <BASENAME>`: (Required) Base name for output files (e.g., `my-transfer`).

**Blockhash/Nonce Options (Choose one group):**

*   *(No option)*: Fetches the latest blockhash from the RPC URL.
*   `--recent-blockhash <HASH>`: Use a specific recent blockhash (Base58). Cannot be used with nonce options.
*   `--nonce-account <PUBKEY> --nonce-authority <PUBKEY>`: Use a durable nonce. Provide the nonce account's public key and its authority's public key. Fetches the current nonce value from the account to use as the blockhash and prepends the `AdvanceNonceAccount` instruction.

### `sign`

Signs an unsigned transaction.

```bash
solana-kms-tool sign [OPTIONS] --signer <KEY_IDENTIFIER> --output-sig <PATH> <UNSIGNED_TX_PATH>
```

**Arguments:**

*   `<UNSIGNED_TX_PATH>`: (Required) Path to the `*.unsigned.solana.json` file.
*   `-k, --signer <KEY_IDENTIFIER>`: (Required) Identifies the signing key:
    *   **Mainnet (Ledger):** The Base58 *public key* of the signer's account on the Ledger.
    *   **Testnet/Devnet:** The *file path* to the signer's keypair JSON file.
*   `-s, --output-sig <PATH>`: (Required) Path where the output partial signature file (`*.sig.json`) will be saved.

### `combine`

Combines partial signatures.

```bash
solana-kms-tool combine [OPTIONS] --unsigned-tx-path <PATH> --output-signed <PATH> --signatures <SIG_PATH>...
```

**Arguments:**

*   `--unsigned-tx-path <PATH>`: (Required) Path to the original `*.unsigned.solana.json` file.
*   `-s, --signatures <SIG_PATH>...`: (Required) One or more paths to the partial signature files (`*.sig.json`).
*   `-f, --output-signed <PATH>`: (Required) Path where the combined signed transaction file (`*.signed.solana.json`) will be saved.

### `broadcast`

Broadcasts a combined signed transaction.

```bash
solana-kms-tool broadcast [OPTIONS] <SIGNED_TX_PATH>
```

**Arguments:**

*   `<SIGNED_TX_PATH>`: (Required) Path to the `*.signed.solana.json` file.

## Instruction File Format (`--instructions`)

The file specified via `--instructions` must be a JSON file containing an array of instruction objects. Each instruction object should follow this structure:

```json
[
  {
    "program_id": "11111111111111111111111111111111", // Program ID (Base58)
    "accounts": [
      {
        "pubkey": "RecipientPublicKeyBase58...", // Account Pubkey (Base58)
        "is_signer": false,
        "is_writable": true
      },
      {
        "pubkey": "SenderPublicKeyBase58...",
        "is_signer": true,                      // This account needs to sign
        "is_writable": true
      }
      // ... more accounts for the instruction ...
    ],
    "data": "03000000e803000000000000" // Instruction data (Hex encoded bytes)
  }
  // ... more instructions in the array ...
]
```

*   `program_id`: The Base58 address of the program to call.
*   `accounts`: An array of account metadata (`AccountMeta`).
    *   `pubkey`: The Base58 address of the account.
    *   `is_signer`: `true` if this account's signature is required for this instruction, `false` otherwise.
    *   `is_writable`: `true` if the program needs to modify this account, `false` otherwise.
*   `data`: The instruction-specific data, encoded as a hexadecimal string.

*(Note: If using durable nonces, the `AdvanceNonceAccount` instruction is automatically prepended by the `generate` command; you only need to provide your main transaction instructions in the JSON file.)*

## Examples

*(Replace `solana-kms-tool` with your actual binary path, and `<...>` placeholders with real values)*

**1. Standard Transfer (Testnet/Devnet - Latest Blockhash)**

```bash
# Assume instructions.json defines an SPL token transfer

# Generate unsigned tx (fetches latest blockhash)
solana-kms-tool --network devnet --rpc-url <RPC_URL> \
  generate \
  --fee-payer <FEE_PAYER_PUBKEY> \
  --instructions ./instructions.json \
  --output-name my-transfer

# Sign using local keypair (assuming fee payer is also instruction signer)
solana-kms-tool --network devnet \
  sign \
  --signer ~/.config/solana/id.json \
  --output-sig ./output/my-transfer.sig.json \
  ./output/my-transfer.unsigned.solana.json

# Combine (only one signature in this case)
solana-kms-tool --network devnet \
  combine \
  --unsigned-tx-path ./output/my-transfer.unsigned.solana.json \
  --signatures ./output/my-transfer.sig.json \
  --output-signed ./output/my-transfer.signed.solana.json

# Broadcast
solana-kms-tool --network devnet --rpc-url <RPC_URL> \
  broadcast \
  ./output/my-transfer.signed.solana.json
```

**2. Durable Nonce Transfer (Testnet/Devnet)**

```bash
# Assume instructions.json defines the main transfer instruction
# Assume Nonce Account: <NONCE_ACCOUNT_PUBKEY>
# Assume Nonce Authority: <NONCE_AUTHORITY_PUBKEY> (e.g., ~/.config/solana/id.json)

# Generate unsigned tx using nonce
solana-kms-tool --network devnet --rpc-url <RPC_URL> \
  generate \
  --fee-payer <FEE_PAYER_PUBKEY> \
  --instructions ./instructions.json \
  --output-name my-nonce-tx \
  --nonce-account <NONCE_ACCOUNT_PUBKEY> \
  --nonce-authority <NONCE_AUTHORITY_PUBKEY>

# Sign using Nonce Authority's keypair (required for AdvanceNonce ix)
solana-kms-tool --network devnet \
  sign \
  --signer ~/.config/solana/id.json \
  --output-sig ./output/my-nonce-tx.nonce-auth.sig.json \
  ./output/my-nonce-tx.unsigned.solana.json

# Sign using Fee Payer's keypair (if different from nonce authority)
# solana-kms-tool --network devnet sign --signer <FEE_PAYER_KEYPATH> ...

# Sign using any other instruction signer's keypair
# solana-kms-tool --network devnet sign --signer <OTHER_SIGNER_KEYPATH> ...

# Combine all required signatures
solana-kms-tool --network devnet \
  combine \
  --unsigned-tx-path ./output/my-nonce-tx.unsigned.solana.json \
  --signatures ./output/my-nonce-tx.nonce-auth.sig.json \
               # ./output/my-nonce-tx.fee-payer.sig.json \ # Add other sigs
               # ./output/my-nonce-tx.other-signer.sig.json
  --output-signed ./output/my-nonce-tx.signed.solana.json

# Broadcast
solana-kms-tool --network devnet --rpc-url <RPC_URL> \
  broadcast \
  ./output/my-nonce-tx.signed.solana.json
```

**3. Multisig Simulation (Testnet/Devnet)**

```bash
# Assume 2 signers required (e.g., SignerA, SignerB) + FeePayer
# Assume instructions require SignerA and SignerB

# Generate (latest blockhash or nonce)
solana-kms-tool --network devnet --rpc-url <RPC_URL> generate ... --output-name my-multisig

# Signer A signs
solana-kms-tool --network devnet sign \
  --signer /path/to/signerA.json \
  --output-sig ./output/my-multisig.signerA.sig.json \
  ./output/my-multisig.unsigned.solana.json

# Signer B signs
solana-kms-tool --network devnet sign \
  --signer /path/to/signerB.json \
  --output-sig ./output/my-multisig.signerB.sig.json \
  ./output/my-multisig.unsigned.solana.json

# Fee Payer signs (if different)
solana-kms-tool --network devnet sign \
  --signer /path/to/feePayer.json \
  --output-sig ./output/my-multisig.feePayer.sig.json \
  ./output/my-multisig.unsigned.solana.json

# Combine all signatures
solana-kms-tool --network devnet combine \
  --unsigned-tx-path ./output/my-multisig.unsigned.solana.json \
  --signatures ./output/my-multisig.signerA.sig.json \
               ./output/my-multisig.signerB.sig.json \
               ./output/my-multisig.feePayer.sig.json \
  --output-signed ./output/my-multisig.signed.solana.json

# Broadcast
solana-kms-tool --network devnet --rpc-url <RPC_URL> broadcast \
  ./output/my-multisig.signed.solana.json
```

**4. Mainnet Offline Workflow (Conceptual)**

1.  **Online:** Run `solana-kms-tool --network mainnet-beta ... generate ...`
    *   This creates `*.unsigned.solana.json` AND `*.solana.bundle.tar.gz`.
2.  **Transfer:** Securely transfer the `*.solana.bundle.tar.gz` to each required signer's **offline** machine (e.g., via encrypted USB).
3.  **Offline (Signer 1):**
    *   Unpack the bundle (`tar -xzf ...`).
    *   Connect Ledger device.
    *   Run `./signing_tools/solana_key_management_tool --network mainnet-beta sign --signer <SIGNER1_LEDGER_PUBKEY> --output-sig signer1.sig.json unsigned_tx.solana.json` (Note: uses the binary *inside* the bundle).
    *   Securely transfer `signer1.sig.json` out (e.g., save to USB).
4.  **Offline (Signer 2):** Repeat step 3 for Signer 2, generating `signer2.sig.json`.
5.  **Online (Coordinator):** Collect all `*.sig.json` files.
6.  **Online:** Run `solana-kms-tool --network mainnet-beta combine ... --signatures signer1.sig.json signer2.sig.json ... --output-signed final.signed.solana.json`
7.  **Online:** Run `solana-kms-tool --network mainnet-beta --rpc-url <MAINNET_RPC> broadcast final.signed.solana.json`

## Important Files Generated

*   `*.unsigned.solana.json`: Contains transaction parameters (fee payer, blockhash/nonce info) and instructions. Used as input for signing.
*   `*.sig.json`: Contains a single signer's public key and their signature for the transaction.
*   `*.signed.solana.json`: Contains the original unsigned transaction data plus an array of all collected partial signatures. Used as input for broadcasting.
*   `*.solana.bundle.tar.gz` (Mainnet Only): An archive containing the unsigned transaction JSON and the tools needed for offline signing (including this compiled binary).

## Ledger Integration Note

The interaction with the Ledger device in the `sign` command is currently **simulated**. To use with a real Ledger, you would need to:
1.  Find and integrate a suitable Rust crate for Solana Ledger communication (e.g., potentially `ledger-solana` if maintained and compatible).
2.  Replace the placeholder code in `src/sign.rs::sign_with_ledger_solana` with actual APDU command exchange logic using the chosen crate.
3.  Build the tool with the necessary Ledger feature flag enabled (if required by the crate).
