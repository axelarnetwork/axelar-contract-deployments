# Solana GMP Upgrade (anchor to anchor)

## Network Status

|                | **Owner**    |
| -------------- | ------------ |
| **Created By** | @nbayindirli |
| **Deployment** | @nbayindirli |

| **Axelar Env**       | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Pending               | TBD        |
| **Stagenet**         | Pending               | TBD        |
| **Testnet**          | Pending               | TBD        |
| **Mainnet**          | Pending               | TBD        |

## Background

This is the Solana GMP upgrade doc for anchor TO anchor programs.

## Deployment

### Deployment Prerequisites

- A GMP/ITS version should already be deployed, per another release. Examples:
  - [2025-09-GMP-anchor](./2025-09-GMP-anchor.md)
  - [2025-09-ITS-anchor](./2025-09-ITS-anchor.md)

1. Ensure all `Deployment Prerequisites` in [2025-09-GMP-v1-upstream](./2025-09-GMP-v1-upstream.md) are already met.

### Deployment Steps

1. Ensure the following environment variables are sourced.

    ```bash
    ENV=<devnet-custom|devnet-amplifier|stagenet|testnet|mainnet>
    CLUSTER=<devnet|mainnet-beta>
    ```

1. Clone the [`axelar-amplifier-solana`](https://github.com/axelarnetwork/axelar-amplifier-solana) repo.

1. Ensure the branch you would like to upgrade to is checked out and cd to the new repo.

1. Compile the Solana programs:

    ```sh
    # Go to the solana directory within the cloned repo
    cd axelar-amplifier-solana

    # Compile the Solana programs
    cargo build-sbf --manifest-path programs/solana-axelar-gateway/Cargo.toml
    cargo build-sbf --manifest-path programs/solana-axelar-gas-service/Cargo.toml
    cargo build-sbf --manifest-path programs/solana-axelar-governance/Cargo.toml
    cargo build-sbf --manifest-path programs/solana-axelar-multicall/Cargo.toml
    cargo build-sbf --manifest-path programs/solana-axelar-operators/Cargo.toml
    cargo build-sbf --manifest-path programs/solana-axelar-memo/Cargo.toml
    cargo build-sbf --manifest-path programs/solana-axelar-its/Cargo.toml

    # Go back
    cd ..
    ```

1. Reassign the follow environment variables for the programs you are upgrading:

    ```sh
    PROGRAM_KEYPAIR_PATH="<path/to/program_keypair.json>"
    PROGRAM_PATH="axelar-amplifier-solana/target/deploy/<program_name>.so"
    PROGRAM_PDA="[program-pda]"

    UPGRADE_AUTHORITY_KEYPAIR_PATH="<path/to/upgrade_authority_keypair.json>"
    ```

    ```bash
    source .env
    ```

1. Ensure Solana CLI is set to the relevant cluster:

   ```bash
   solana config set --url $CLUSTER
   ```

1. Upgrade the programs:

    ```sh
    anchor upgrade --provider.wallet $UPGRADE_AUTHORITY_KEYPAIR_PATH --provider.cluster $CLUSTER -p $PROGRAM_PDA $PROGRAM_PATH -- --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH
    ```

1. Verify the programs:

    > [!NOTE]
    > Verification is **only possible in mainnet**. If deploying for test environments you can skip this step.

    ```bash
    anchor verify -p [solana_axelar_program_name] --provider.cluster $CLUSTER $(solana address -k $PROGRAM_KEYPAIR_PATH) -- --no-default-features --features $ENV
    ```

## Checklist

- [ ] Re-run `GMP` test transaction (see final section in original deployment docs ([1](./2025-09-GMP-anchor.md), [2](./2025-09-ITS-anchor.md)).
- [ ] Run the [e2e repository](https://github.com/eigerco/axelar-solana-e2e) pipeline.
