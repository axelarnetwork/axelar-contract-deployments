# Solana RPC Node Setup

This guide covers setting up a Solana RPC node on **devnet**, **testnet**, or **mainnet-beta**.

## Prerequisites

Follow the official Anza validator guide for initial setup:

1. [System Requirements](https://docs.anza.xyz/operations/requirements) — hardware, OS, and networking requirements per cluster.
2. [System Tuning](https://docs.anza.xyz/operations/best-practices/general#system-tuning) — file descriptor limits, kernel parameters, and other OS-level configuration.
3. [Install the Agave CLI](https://docs.anza.xyz/cli/install) — install the Agave validator and Solana CLI tools.

Once these steps are complete, continue below with the RPC-specific configuration.

## Storage Layout

This setup assumes the node has dedicated storage mounted at:

- `/mnt/ledger`
- `/mnt/accounts`
- `/mnt/snapshots`

Recommended layout:
- 1 dedicated SSD for ledger
- 1 dedicated SSD for accounts
- 1 dedicated SSD for snapshots
- optional separate OS disk

Notes:
- A separate OS disk is preferred but not strictly required.
- The OS may share the ledger disk, but performance is generally better when ledger has its own disk.
- Accounts and ledger can share a disk, but this is not recommended due to high IOPS pressure.

## Cluster Reference

| | Devnet | Testnet | Mainnet-Beta |
|--------------------|--------------------------------------------|--------------------------------------------|-----------------------------------------------------|
| **Genesis Hash** | `EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG` | `4uhcVJyU9pJkvQyS88uRDiswHXSCkY3zQawwpjk2NsNY` | `5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d` |
| **Entrypoints** | `entrypoint{,2,3,4}.devnet.solana.com:8001` | `entrypoint{,2,3}.testnet.solana.com:8001` | `entrypoint{,2,3,4,5}.mainnet-beta.solana.com:8001` |
| **Ledger limit** | 200000000 | 200000000 | 400000000 |

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

## Generate Validator Identity

```bash
sudo mkdir -p /home/sol/solana/keys
sudo mkdir -p /var/log/solana
sudo chown -R sol:sol /home/sol/solana
sudo chown -R sol:sol /var/log/solana
chmod 700 /home/sol/solana/keys
```

```bash
solana-keygen new --outfile /home/sol/solana/keys/validator-keypair.json --no-bip39-passphrase
```

## Run RPC Node

Replace the `<CLUSTER-SPECIFIC>` lines below with the entrypoints, genesis hash, and known validators for your cluster from the [Cluster Reference](#cluster-reference) table above.

```bash
agave-validator \
  --identity /home/sol/solana/keys/validator-keypair.json \
  --no-voting \
  --ledger /mnt/ledger \
  --accounts /mnt/accounts \
  --snapshots /mnt/snapshots \
  --log /var/log/solana/validator.log \
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
  --minimal-snapshot-download-speed 100000000 \
  --enable-rpc-transaction-history \
  --full-rpc-api
```

**Optional:** `--minimal-snapshot-download-speed 100000000` picks a faster peer for downloading snapshots.

## Setup systemd Service

Create `/etc/systemd/system/agave-validator.service`, replacing the `<CLUSTER-SPECIFIC>` lines the same way as above:

```ini
[Unit]
Description=Agave Validator
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=sol
Group=sol
LimitNOFILE=2000000
LimitNPROC=2000000
LimitMEMLOCK=infinity
Restart=always
RestartSec=5
Environment=RUST_BACKTRACE=1
ExecStart=/usr/local/bin/agave-validator \
  --identity /home/sol/solana/keys/validator-keypair.json \
  --no-voting \
  --ledger /mnt/ledger \
  --accounts /mnt/accounts \
  --snapshots /mnt/snapshots \
  --log /var/log/solana/validator.log \
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
  --minimal-snapshot-download-speed 100000000 \
  --enable-rpc-transaction-history \
  --full-rpc-api
ExecStop=/bin/kill -s INT $MAINPID
TimeoutStopSec=300

[Install]
WantedBy=multi-user.target
```

Reload and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable agave-validator
sudo systemctl start agave-validator
```

## Check Progress
```bash
solana catchup $(solana-keygen pubkey /home/sol/solana/keys/validator-keypair.json) --our-localhost 8899
```

## Check Logs

```bash
tail -f /var/log/solana/validator.log | grep -E "Downloading|downloaded|snapshot|RPC node root slot|Loading bank|Processing ledger|caught up"
```
