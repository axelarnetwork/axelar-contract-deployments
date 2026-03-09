# Solana RPC Node Setup

## Overview

Running a dedicated Solana RPC node for the Axelar amplifier stack (ampd handler + relayer). Public RPC endpoints have aggressive rate limits that make them unsuitable for production relaying.

The amplifier stack requires WebSocket `logsSubscribe`, `getSignaturesForAddress` for Axelar program accounts (Gateway, Gas Service, ITS), `getTransaction` with full metadata, and standard write-path RPCs.

## Cluster Reference

|                    | Devnet                                     | Testnet                                    | Mainnet-Beta                                        |
|--------------------|--------------------------------------------|--------------------------------------------|-----------------------------------------------------|
| **Genesis Hash**   | `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG` | `4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY` | `5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d` |
| **Entrypoints**    | `entrypoint{,2,3,4}.devnet.solana.com:8001` | `entrypoint{,2,3}.testnet.solana.com:8001` | `entrypoint{,2,3,4,5}.mainnet-beta.solana.com:8001` |
| **CPU**            | 16+ cores                                  | 16+ cores                                  | 32+ cores                                           |
| **RAM**            | 64 GB+                                     | 64 GB+                                     | 128 GB+ (256 GB ideal)                              |
| **Disk**           | 2 TB+ NVMe                                 | 2 TB+ NVMe                                 | 4 TB+ NVMe                                          |
| **Network**        | 1 Gbps                                     | 1 Gbps                                     | 1 Gbps min, 10 Gbps recommended                     |
| **Ledger limit**   | 200000000                                  | 200000000                                  | 400000000                                            |

All clusters need Ubuntu 22.04+/Debian 12+, a dedicated NVMe (not shared with OS), and UDP ports 8000-8030 open for gossip/turbine.

### Known Validators

<details>
<summary>Devnet</summary>

```
dv1ZAGvdsz5hHLwWXsVnM94hWf1pjbKVau1QVkaMJ92
dv2eQHeP4RFrJZ6UeiZWoc3XTtmtZCUKEUFr7GMkKHb
dv3qDFk1DTF36Z62bNvrCXe9sKATA6xvVy6A798xxAS
dv4ACNkpYPcE3aKmYDqZm9G5EB3J4MRoeE7WNDRBVJB
```
</details>

<details>
<summary>Testnet</summary>

```
5D1fNXzvv5NjV1ysLjirC4WY92RNsVH18vjmcszZd8on
dv1ZAGvdsz5hHLwWXsVnM94hWf1pjbKVau1QVkaMJ92
dv2eQHeP4RFrJZ6UeiZWoc3XTtmtZCUKEUFr7GMkKHb
dv4ACNkpYPcE3aKmYDqZm9G5EB3J4MRoeE7WNDRBVJB
```
</details>

<details>
<summary>Mainnet-Beta</summary>

```
7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2
GdnSyH3YtwcxFvQrVVJMm1JhTS4QVX7MFsX56uJLUfiZ
DE1bawNcRJB9rVm3buyMVfr8mBEoyyu73NBovf2oXJsJ
CakcnaRDHka2gXyfbEd2d3xsvkJkqsLw2akB3zsN1D2S
```
</details>

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
CLUSTER=devnet  # or: testnet, mainnet-beta
solana config set --url https://api.${CLUSTER}.solana.com
```

## Download Snapshot

The full snapshot can be 40-80+ GB depending on the cluster. The built-in snapshot fetcher downloads at ~10-19 MB/s, and if it takes too long the validator considers the snapshot "too old" and restarts the download in a loop. To avoid this, download the snapshot manually before starting:

```bash
# Find a peer's snapshot URL (the redirect gives you the actual filename with slot and hash)
curl -sL -o /dev/null -w "%{url_effective}\n" "http://<PEER_IP>:8899/snapshot.tar.bz2"
# Example output: http://<PEER_IP>:8899/snapshot-446616651-CBVNvzLcoWNDPmJVRiW1LX3jCr3QumfDYSvgiyhsqVjR.tar.zst

# Download directly into the snapshots directory with the correct filename
cd $SOLANA_DIR/snapshots
wget -O "snapshot-<SLOT>-<HASH>.tar.zst" "http://<PEER_IP>:8899/snapshot-<SLOT>-<HASH>.tar.zst"
```

For devnet, some known snapshot peers (check availability):
- `109.94.99.153` (dv1), `198.13.130.39` (dv2), `64.130.41.138` (dv3), `64.130.33.238` (dv4)

For testnet/mainnet, find peers via `solana gossip` or community resources.

You can test download speed first:
```bash
curl -sL --max-time 6 -o /dev/null -w "%{speed_download} B/s\n" "http://<PEER_IP>:8899/snapshot.tar.bz2"
```

## Validator Startup Script

Set the cluster-specific variables, then generate the script. See the [Cluster Reference](#cluster-reference) table for values.

```bash
# --- Cluster-specific config (change these) ---
GENESIS_HASH=EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG  # see table
LEDGER_LIMIT=200000000                                       # 400000000 for mainnet

ENTRYPOINTS=(
    entrypoint.devnet.solana.com:8001
    entrypoint2.devnet.solana.com:8001
    entrypoint3.devnet.solana.com:8001
    entrypoint4.devnet.solana.com:8001
)

KNOWN_VALIDATORS=(
    dv1ZAGvdsz5hHLwWXsVnM94hWf1pjbKVau1QVkaMJ92
    dv2eQHeP4RFrJZ6UeiZWoc3XTtmtZCUKEUFr7GMkKHb
    dv3qDFk1DTF36Z62bNvrCXe9sKATA6xvVy6A798xxAS
    dv4ACNkpYPcE3aKmYDqZm9G5EB3J4MRoeE7WNDRBVJB
)

# --- Generate script (no changes needed below) ---
{
echo '#!/usr/bin/env bash'
echo 'set -e'
echo ''
echo 'exec agave-validator \'
echo '    --identity /mnt/solana-ssd/solana/validator-keypair.json \'
echo '    --no-voting \'
echo '    --ledger /mnt/solana-ssd/solana/ledger \'
echo '    --accounts /mnt/solana-ssd/solana/accounts \'
echo '    --snapshots /mnt/solana-ssd/solana/snapshots \'
echo '    --log /mnt/solana-ssd/solana/log/validator.log \'
for ep in "${ENTRYPOINTS[@]}"; do
    echo "    --entrypoint $ep \\"
done
echo "    --expected-genesis-hash $GENESIS_HASH \\"
for kv in "${KNOWN_VALIDATORS[@]}"; do
    echo "    --known-validator $kv \\"
done
echo '    --rpc-port 8899 \'
echo '    --rpc-bind-address 0.0.0.0 \'
echo '    --private-rpc \'
echo '    --dynamic-port-range 8000-8030 \'
echo '    --wal-recovery-mode skip_any_corrupted_record \'
echo "    --limit-ledger-size $LEDGER_LIMIT \\"
echo '    --enable-rpc-transaction-history \'
echo '    --full-rpc-api \'
echo '    --no-os-network-limits-test \'
echo '    --no-snapshot-fetch \'
echo '    --account-index program-id \'
echo '    --maximum-full-snapshots-to-retain 2 \'
echo '    --maximum-incremental-snapshots-to-retain 4'
} > $SOLANA_DIR/validator.sh
chmod +x $SOLANA_DIR/validator.sh
```

**Important:** `--no-snapshot-fetch` tells the validator to use the snapshot you downloaded manually instead of trying to fetch one from peers. After the first successful boot you can remove this flag for subsequent restarts (the node will have its own local snapshots by then).
