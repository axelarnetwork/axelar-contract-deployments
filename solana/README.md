# Solana smart contracts

## Development setup

### Backend

```bash
cargo install cargo-make
cargo make

# Lint
cargo make check

# Check for unused deps
cargo make unused-deps

# Run the tests
cargo make test
cargo make coverage

# Check if the CI tasks will pass
cargo make local-ci

# Audit deps
cargo make audit

# Format the code
cargo make fmt
```
