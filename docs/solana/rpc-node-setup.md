# Solana RPC Node Setup

## Overview

Running a dedicated Solana devnet RPC node for the Axelar amplifier stack (ampd handler + relayer). Public RPC endpoints have aggressive rate limits that make them unsuitable for production relaying.

The amplifier stack requires WebSocket `logsSubscribe`, `getSignaturesForAddress` for Axelar program accounts (Gateway, Gas Service, ITS), `getTransaction` with full metadata, and standard write-path RPCs.

## Prerequisites (Devnet)

- **OS:** Ubuntu 22.04+ / Debian 12+
- **CPU:** 16+ cores (32 recommended)
- **RAM:** 64GB+
- **Disk:** 2TB+ NVMe SSD (dedicated, not shared with OS)
- **Network:** 1Gbps+ with UDP ports 8000-8030 open (for gossip/turbine)

## Install Solana CLI (Agave)

```bash
sh -c "$(curl -sSfL https://release.anza.xyz/stable/install)"
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
agave-validator --version
```

Add the PATH export to your `~/.bashrc` or `~/.profile`.

## Setup

```bash
SOLANA_DIR=/mnt/solana-ssd/solana
mkdir -p $SOLANA_DIR/{ledger,accounts,snapshots,log}
```

## Generate Validator Identity

```bash
solana-keygen new --outfile $SOLANA_DIR/validator-keypair.json --no-bip39-passphrase
```

This identity is only used for gossip protocol participation (no voting, no staking).

## Configure Solana CLI

```bash
solana config set --url https://api.devnet.solana.com
```

## Download Snapshot

The devnet full snapshot is ~43-56GB. The built-in snapshot fetcher downloads at ~10-19MB/s, and if it takes too long the validator considers the snapshot "too old" and restarts the download in a loop. To avoid this, download the snapshot manually before starting:

```bash
# Find a known validator's snapshot URL
# The redirect gives you the actual filename with slot and hash
curl -sL -o /dev/null -w "%{url_effective}\n" "http://109.94.99.153:8899/snapshot.tar.bz2"
# Example output: http://109.94.99.153:8899/snapshot-446616651-CBVNvzLcoWNDPmJVRiW1LX3jCr3QumfDYSvgiyhsqVjR.tar.zst

# Download directly into the snapshots directory with the correct filename
cd $SOLANA_DIR/snapshots
wget -O "snapshot-<SLOT>-<HASH>.tar.zst" "http://<PEER_IP>:8899/snapshot-<SLOT>-<HASH>.tar.zst"
```

Alternative known validator IPs for devnet (check availability):
- `109.94.99.153` (dv1)
- `198.13.130.39` (dv2)
- `64.130.41.138` (dv3)
- `64.130.33.238` (dv4)

You can test download speed first:
```bash
curl -sL --max-time 6 -o /dev/null -w "%{speed_download} B/s\n" "http://<PEER_IP>:8899/snapshot.tar.bz2"
```

## Validator Startup Script

```bash
cat > $SOLANA_DIR/validator.sh << 'SCRIPT'
#!/usr/bin/env bash
set -e

exec agave-validator \
    --identity /mnt/solana-ssd/solana/validator-keypair.json \
    --no-voting \
    --ledger /mnt/solana-ssd/solana/ledger \
    --accounts /mnt/solana-ssd/solana/accounts \
    --snapshots /mnt/solana-ssd/solana/snapshots \
    --log /mnt/solana-ssd/solana/log/validator.log \
    --entrypoint entrypoint.devnet.solana.com:8001 \
    --entrypoint entrypoint2.devnet.solana.com:8001 \
    --entrypoint entrypoint3.devnet.solana.com:8001 \
    --entrypoint entrypoint4.devnet.solana.com:8001 \
    --expected-genesis-hash EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG \
    --known-validator dv1ZAGvdsz5hHLwWXsVnM94hWf1pjbKVau1QVkaMJ92 \
    --known-validator dv2eQHeP4RFrJZ6UeiZWoc3XTtmtZCUKEUFr7GMkKHb \
    --known-validator dv3qDFk1DTF36Z62bNvrCXe9sKATA6xvVy6A798xxAS \
    --known-validator dv4ACNkpYPcE3aKmYDqZm9G5EB3J4MRoeE7WNDRBVJB \
    --rpc-port 8899 \
    --rpc-bind-address 0.0.0.0 \
    --private-rpc \
    --dynamic-port-range 8000-8030 \
    --wal-recovery-mode skip_any_corrupted_record \
    --limit-ledger-size 200000000 \
    --enable-rpc-transaction-history \
    --full-rpc-api \
    --no-os-network-limits-test \
    --no-snapshot-fetch \
    --account-index program-id \
    --maximum-full-snapshots-to-retain 2 \
    --maximum-incremental-snapshots-to-retain 4
SCRIPT
chmod +x $SOLANA_DIR/validator.sh
```

**Important:** `--no-snapshot-fetch` tells the validator to use the snapshot you downloaded manually in the Download Snapshot section instead of trying to fetch one from peers. After the first successful boot you can remove this flag for subsequent restarts (the node will have its own local snapshots by then).
