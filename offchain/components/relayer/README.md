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
$ ./target/release/relayer --config <path to configuration file>
```

### Configuration

The Relayer is configured by a TOML file, which path is passed in the `--config` CLI argument.

Example:

```toml
[axelar_to_solana.approver]
rpc = "http://0.0.0.1/"

[axelar_to_solana.includer]
rpc = "http://0.0.0.2/"
keypair = "< the Relayer's secret key in base-58 format >"
gateway_address = "1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM"
gateway_config_address = "1111111ogCyDbaRMvkdsHB3qfdyFYaG1WtRUAfdh"

[solana_to_axelar.sentinel]
gateway_address = "1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM"
rpc = "http://0.0.0.3/"

[solana_to_axelar.verifier]
rpc = "http://0.0.0.4/"

[database]
url = "postgres://user:password@localhost:5432/dbname"

[health_check]
bind_addr = "127.0.0.1:8000"
```

### Partial operation

The Relayer can be configured to operate on a single transport direction by ommiting either the
`axelar_to_solana` or the `solana_to_axelar` TOML tables in the configuration file.


### Reference secret key in an environment variable

The `axelar_to_solana.includer.keypair` variable can be fetched from an environment variable if its
value is prefixed by a dollar sign an the environment variable name.

Example:
```toml
[axelar_to_solana.includer]
rpc = "http://0.0.0.2/"
keypair = "$RELAYER_SECRET_KEY"
gateway_address = "1111111QLbz7JHiBTspS962RLKV8GndWFwiEaqKM"
gateway_config_address = "1111111ogCyDbaRMvkdsHB3qfdyFYaG1WtRUAfdh"
```

The configuration above will fetch the `keypair` value from the `RELAYER_SECRET_KEY` environment variable.
