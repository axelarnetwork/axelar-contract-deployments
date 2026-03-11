# Solana Mainnet RPC Node Setup

## Assumptions
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


## System Tuning
```bash
# Increase file descriptor limits
sudo bash -c 'cat >> /etc/security/limits.conf <<EOF
* soft nofile 1000000
* hard nofile 1000000
EOF'

# Kernel parameters
sudo bash -c 'cat >> /etc/sysctl.conf <<EOF
vm.max_map_count=1000000
net.core.rmem_default=134217728
net.core.rmem_max=134217728
net.core.wmem_default=134217728
net.core.wmem_max=134217728
EOF'

sudo sysctl -p
```

## Install Agave
```bash
sudo apt update
sudo apt install -y \
  git curl build-essential pkg-config libssl-dev libudev-dev clang cmake make \
  llvm libclang-dev protobuf-compiler libprotobuf-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

git clone https://github.com/anza-xyz/agave.git
cd agave
git checkout v3.1.10
cargo build --release --bin agave-validator

sudo cp ./target/release/agave-validator /usr/local/bin/
agave-validator --version
```

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

## Run RPC node

```bash
agave-validator \
  --identity /home/sol/solana/keys/validator-keypair.json \
  --no-voting \
  --ledger /mnt/ledger \
  --accounts /mnt/accounts \
  --snapshots /mnt/snapshots \
  --log /var/log/solana/validator.log \
  --entrypoint entrypoint.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint2.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint3.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint4.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint5.mainnet-beta.solana.com:8001 \
  --expected-genesis-hash 5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d \
  --known-validator 7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2 \
  --known-validator GdnSyH3YtwcxFvQrVVJMm1JhTS4QVX7MFsX56uJLUfiZ \
  --known-validator DE1bawNcRJB9rVm3buyMVfr8mBEoyyu73NBovf2oXJsJ \
  --known-validator CakcnaRDHka2gXyfbEd2d3xsvkJkqsLw2akB3zsN1D2S \
  --rpc-port 8899 \
  --rpc-bind-address 0.0.0.0 \
  --private-rpc \
  --dynamic-port-range 8000-8025 \
  --wal-recovery-mode skip_any_corrupted_record \
  --limit-ledger-size \
  --full-rpc-api \
  --no-os-network-limits-test \
  --maximum-full-snapshots-to-retain 2 \
  --maximum-incremental-snapshots-to-retain 4 \
  --minimal-snapshot-download-speed 100000000
```

**Optional**
Set `--minimal-snapshot-download-speed 100000000` to pick a faster peer for downloading snapshots.

## Setup systemd service

Paste the following in `/etc/systemd/system/agave-validator.service`

```
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
  --entrypoint entrypoint.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint2.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint3.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint4.mainnet-beta.solana.com:8001 \
  --entrypoint entrypoint5.mainnet-beta.solana.com:8001 \
  --expected-genesis-hash 5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d \
  --known-validator 7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2 \
  --known-validator GdnSyH3YtwcxFvQrVVJMm1JhTS4QVX7MFsX56uJLUfiZ \
  --known-validator DE1bawNcRJB9rVm3buyMVfr8mBEoyyu73NBovf2oXJsJ \
  --known-validator CakcnaRDHka2gXyfbEd2d3xsvkJkqsLw2akB3zsN1D2S \
  --rpc-port 8899 \
  --rpc-bind-address 0.0.0.0 \
  --private-rpc \
  --dynamic-port-range 8000-8025 \
  --wal-recovery-mode skip_any_corrupted_record \
  --limit-ledger-size \
  --full-rpc-api \
  --no-os-network-limits-test \
  --maximum-full-snapshots-to-retain 2 \
  --maximum-incremental-snapshots-to-retain 4 \
  --minimal-snapshot-download-speed 100000000
ExecStop=/bin/kill -s INT $MAINPID
TimeoutStopSec=300

[Install]
WantedBy=multi-user.target
```

Reload service
```bash
sudo systemctl daemon-reload
sudo systemctl enable agave-validator
sudo systemctl start agave-validator
```



## Check progress
```bash
solana catchup $(solana-keygen pubkey solana/keys/validator-keypair.json) --our-localhost 8899
```

## Check logs

```bash
tail -f /var/log/solana/validator.log | grep -E "Downloading|downloaded|snapshot|RPC node root slot|Loading bank|Processing ledger|caught up"
```
