# Solana RPC Node Setup

This guide covers setting up a Solana RPC node on **devnet**, **testnet**, or **mainnet-beta**.

## Prerequisites

Follow the official Anza validator guide for initial setup:

1. [System Requirements](https://docs.anza.xyz/operations/requirements) — hardware, OS, and networking requirements per cluster.
2. [Install the Agave CLI](https://docs.anza.xyz/cli/install) — install the Agave validator and Solana CLI tools.
3. [Setup a Validator](https://docs.anza.xyz/operations/setup-a-validator) — follow the initial setup steps (identity keypair, hard drive setup, system tuning). Skip any consensus/voting-related steps since we are running an RPC node.
4. [Setup an RPC Node](https://docs.anza.xyz/operations/setup-an-rpc-node) — cluster entrypoints, known validators, and base startup configuration.

## RPC-Specific Flags

When running `agave-validator`, use these flags for RPC operation:

```bash
--identity /path/to/validator/keypair.json \
--no-voting \
--ledger /path/to/ledger \
--accounts /path/to/accounts \
--snapshots /path/to/snapshots \
--log /path/to/validator.log \
--entrypoint <ENTRYPOINT_1> \
--entrypoint <ENTRYPOINT_2> \
--entrypoint <ENTRYPOINT_3> \
... \
--expected-genesis-hash <GENESIS_HASH> \
--known-validator <VALIDATOR_1> \
--known-validator <VALIDATOR_2> \
... \
--rpc-port 8899 \
--rpc-bind-address 0.0.0.0 \
--private-rpc \
--dynamic-port-range 8000-8025 \
--wal-recovery-mode skip_any_corrupted_record \
--limit-ledger-size \
--no-os-network-limits-test \
--maximum-full-snapshots-to-retain 2 \
--maximum-incremental-snapshots-to-retain 4 \
--enable-rpc-transaction-history \
--full-rpc-api \
--only-known-rpc
```

**Optional:** `--minimal-snapshot-download-speed 100000000` picks a faster peer for downloading snapshots.
