# Test Suite - Solana Gateway Client
Part of development tools aka "Solana client for Solana Gateway"

## How to run?
First of all the requirement is to have installed `solana-cli`.

```bash
RUST_LOG=debug cargo run -- --payload "somepayload" --destination-chain "somechain" --destination-contract-address "0x999991888887653456765445676544567654567765" --solana-payer-path "~/.config/solana/id.json"
```