# Axelar-Solana Relayer

## Building

1. Fetch all git submodules.
```sh
$ git submodule update --remote
```

1. Compile the `relayer` package.
```sh
$ cargo build --release --package relayer
```

## Running

```sh
$ ./target/release/relayer
```

### Configuration

All the following environment variables must be set when running the Relayer:

- `RELAYER_DATABASE_URL`: PostgreSQL connection string to the Relayer database.
- `RELAYER_AXELAR_APPROVER_URL`: URL to the Amplifier API used to obtain approved messages
- `RELAYER_SOLANA_INCLUDER_RPC`: URL for the Solana JSON RPC endpoint used to submit transactions.
- `RELAYER_SOLANA_INCLUDER_KEYPAIR`: Keypair (private key) used to sign Solana transactions, in base58 format.
- `RELAYER_SENTINEL_GATEWAY_ADDRESS`: The address for the Solana Gateway, in base58 format.
- `RELAYER_SENTINEL_GATEWAY_CONFIG_ADDRESS`: The address for the Solana Gateway Root configuration PDA , in base58 format.
- `RELAYER_SENTINEL_RPC`: URL for the Solana JSON RPC endpoint used to monitor the Gateway program.
- `RELAYER_VERIFIER_RPC`: URL for the Amplifier API used to verify messages originated from the Solana Gateway.
- `RELAYER_HEALTHCHECK_BIND_ADDR`: Socket address to be used by the health check HTTP server, in IPv4 or IPv6 format.
