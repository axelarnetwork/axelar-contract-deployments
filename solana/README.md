# Solana smart contracts

## Development setup

1. [install rust](https://www.rust-lang.org/tools/install)
2. [install solana tool suite](https://docs.solanalabs.com/cli/install)
3. [install foundry & build EVM contracts](../evm-contracts/README.md)

```bash
cargo install cargo-make
cargo make

# Lint
cargo make check

# Check for unused deps
cargo make unused-deps

# Run the tests
cargo make test

# Check if the CI tasks will pass
cargo make local-ci

# Audit deps
cargo make audit

# Format the code
cargo make fmt
```
