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
    UPGRADE_AUTHORITY_KEYPAIR_PATH="<path/to/upgrade_authority_keypair.json>"
    ```

    ```bash
    source .env
    ```

1. Ensure Solana CLI is set to the relevant cluster:

   ```bash
   solana config set --url $CLUSTER
   ```

1. Upgrade the programs using the new version:

    ```sh
    # Upgrade Gateway
    solana/cli upgrade \
        --program gateway \
        --version <VERSION> \
        --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH

    # Upgrade Gas Service
    solana/cli upgrade \
        --program gas-service \
        --version <VERSION> \
        --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH

    # Upgrade Governance
    solana/cli upgrade \
        --program governance \
        --version <VERSION> \
        --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH

    # Upgrade Operators
    solana/cli upgrade \
        --program operators \
        --version <VERSION> \
        --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH

    # Upgrade ITS
    solana/cli upgrade \
        --program its \
        --version <VERSION> \
        --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH
    ```

    > [!NOTE]
    > Replace `<VERSION>` with either a semver (e.g., `0.1.7`) to download from GitHub releases, or a commit hash (e.g., `12e6126`) to download from R2.

## Checklist

- [ ] Re-run `GMP` test transaction (see final section in original deployment docs ([1](./2025-09-GMP-anchor.md), [2](./2025-09-ITS-anchor.md)).
- [ ] Run the [e2e repository](https://github.com/eigerco/axelar-solana-e2e) pipeline.
