# Axelar-Solana Relayer

## Building

### 1. Fetch all git submodules.
```sh
$ git submodule update --remote
```

### 2. Update the static/prepared `sqlx` query files.

> Note: this step is required only if sqlx queries were modified.

To update the `.sqlx` query files, use this command with an active connection to a fully migrated database:

```sh
$ DATABASE_URL="<postgres connection string>" cargo sqlx prepare
```

For full details, refer to the official[`sqlx` documentation on static queries](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query).

### 3. Compile the `relayer` package.

```sh
$ cargo build --release --package relayer
```

To build the project offline, unset `DATABASE_URL` or set `SQLX_OFFLINE=true`.

For full details, refer to the official [`sqlx` documentation on offline building](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#force-building-in-offline-mode).

## Running

```sh
$ ./target/release/relayer --config <path to configuration file>
```

## Database

### Migrations

The Relayer depends on an active connection to a PostgreSQL database to keep track of its progress.

Ensure all necessary migrations are applied to the database before starting the Relayer.

To apply the required migrations, execute the following command:
```sh
$ sqlx migrate run --database-url "$POSTGRES_DATABASE_URL"
```

For full details, refer to the official [`sqlx` documentation on migrations](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md).

### Local PostgreSQL Setup with Docker

#### 1. Pull the PostgreSQL Docker image
```bash
docker pull postgres
```

#### 2. Start a PostgreSQL container
```bash
docker run --name local-postgres -e POSTGRES_USER="${DB_USER}" -e POSTGRES_PASSWORD="${DB_PASSWD}" -e POSTGRES_DB="${DB_NAME}" -p 5432:5432 -d postgres
```
You can now use the following connection string to connect to the database:
```
postgresql://${DB_USER}:${DB_PASSWD}@localhost:5432/${DB_NAME}
```

This connection string can be used both in the `database.url` configuration entry an as the value for `DATABASE_URL` when preparing `.sqlx` static query files.

## Configuration

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
