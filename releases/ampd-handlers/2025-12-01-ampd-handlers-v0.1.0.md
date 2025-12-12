# Ampd Handlers v0.1.0

|                | **Owner**                         |
| -------------- | --------------------------------- |
| **Created By** | @eguajardo <edwin@interoplabs.io> |
| **Deployment** | TBD                               |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

## Releases

- [EVM handlers](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/evm-handler-v0.1.0)
- [Sui handlers](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/sui-handler-v0.1.0)
- [Stellar handlers](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/stellar-handler-v0.1.0)
- [XRPL handlers](TBD)
- [Solana handlers](TBD)

## Background

This is the first release of the new modular handler architecture for Axelar verifiers. Previously, all chain-specific verification logic was bundled into a single `ampd` daemon. With this release, handlers are now deployed as independent services.

### What's Changing

| Aspect                    | Previous Setup                                                                                                                         | New Setup                                                                                                                                                              |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Architecture**          | Single `ampd` binary handled all blockchain verification                                                                               | `ampd` daemon runs as the core service with independent handler binaries for each chain                                                                                |
| **Reliability**           | If a handler crashed, the entire daemon would crash                                                                                    | Handlers operate independently; a crash in one chain handler doesn't affect others                                                                                     |
| **Handler Configuration** | Three different handlers configured per chain in `ampd` config: message verification, verifier set verification, and multisig handling | Each chain binary runs voting and multisig handlers within the same process. Requires two config files: base config (shared by all handlers) and chain-specific config |

### Deployment Impact

Verifiers will need to deploy and maintain separate handler services alongside the core `ampd` daemon.

**Important:** Handler processes should be configured with automatic restart policies to ensure continuous operation. This ensures that if a handler process crashes, it will automatically restart without manual intervention, maintaining uninterrupted verification for that chain.

## AMPD Config Changes

**Important:** Before making changes, backup your handler configuration from the `ampd` config.toml file (the `[[handlers]]` entries). Some values will be needed to configure the new modular handlers. After backing up, remove all `[[handlers]]` entries from `ampd` config.toml to disable the legacy handlers, otherwise they will conflict with the new modular handlers.

### Configuring the gRPC Server

1. Locate the `[grpc]` section in your `ampd` config.toml file, or add it if it doesn't exist:

```toml
[grpc]
ip_addr = "127.0.0.1"
port = "9090"
global_concurrency_limit = "1024"
concurrency_limit_per_connection = "32"
request_timeout = "30s"
blockchain_service.chains = []
```

2. Update the `ip_addr` and `port` values to match the IP address and port where the gRPC server should listen.

3. Remove the `blockchain_service.chains = []` line. Instead, add a `[[grpc.blockchain_service.chains]]` section for each chain you support. The contract addresses must correspond to the deployed CosmWasm contracts for that chain:

```toml
[[grpc.blockchain_service.chains]]
chain_name = "[CHAIN_NAME]"
voting_verifier = "[CHAIN_VOTING_VERIFIER_CONTRACT_ADDRESS]"
multisig_prover = "[CHAIN_MULTISIG_PROVER_CONTRACT_ADDRESS]"
multisig = "[CHAIN_MULTISIG_CONTRACT_ADDRESS]"
```

Contract addresses can be found in the [axelar-contract-deployments repository](https://github.com/axelarnetwork/axelar-contract-deployments/tree/main/axelar-chains-config/info).

## Modular Handler Config

Each handler binary requires two configuration files:

1. **Base config** (`config.toml`) - shared settings required by all handlers
2. **Chain-specific config** - custom parameters for the specific blockchain

By default, the handler binaries look for these configuration files in the current working directory. You can specify a custom configuration directory using the `--config-dir` (or `-c`) argument when starting the handler:

```bash
./evm-handler --config-dir ~/.ampd/evm/
./sui-handler --config-dir ~/.ampd/sui/
./stellar-handler --config-dir ~/.ampd/stellar/
```

#### Example Directory Structure

When using separate directories for each chain handler, organize your configuration as follows:

```
~/.ampd/
├── evm/
│   ├── config.toml                # Base config for EVM handler
│   └── evm-handler-config.toml    # EVM-specific config
├── sui/
│   ├── config.toml                # Base config for Sui handler
│   └── sui-handler-config.toml    # Sui-specific config
├── stellar/
│   ├── config.toml                # Base config for Stellar handler
│   └── stellar-handler-config.toml # Stellar-specific config
├── xrpl/
│   ├── config.toml                # Base config for XRPL handler (when available)
│   └── xrpl-handler-config.toml   # XRPL-specific config (when available)
└── solana/
    ├── config.toml                # Base config for Solana handler (when available)
    └── solana-handler-config.toml # Solana-specific config (when available)
```

Each handler directory contains both the base `config.toml` and its corresponding chain-specific configuration file.

### Base Config

Create a file named `config.toml` in your configuration directory with the following content. Replace the placeholder values with your actual verifier and network details. Optional parameters with default values are omitted for simplicity.

```toml
ampd_url = "[YOUR_AMPD_GRPC_URL]"
chain_name = "[YOUR_CHAIN_NAME]"
```

**Note:** The `ampd_url` should match the gRPC server address configured in your `ampd` config (e.g., `http://127.0.0.1:9090`).

### Chain-Specific Config

Create the appropriate configuration file for your chain handler in your configuration directory. The parameter values should match those used in your legacy `ampd` handler configuration. Optional parameters with default values are omitted for simplicity.

#### EVM Handler Config

Create a file named `evm-handler-config.toml` in your configuration directory:

```toml
rpc_url = "[YOUR_EVM_CHAIN_RPC_URL]"
finalization = "RPCFinalizedBlock" # or "ConfirmationHeight"
```

#### Sui Handler Config

Create a file named `sui-handler-config.toml` in your configuration directory:

```toml
rpc_url = "[YOUR_SUI_CHAIN_RPC_URL]"
```

#### Stellar Handler Config

Create a file named `stellar-handler-config.toml` in your configuration directory:

```toml
rpc_url = "[YOUR_STELLAR_CHAIN_RPC_URL]"
```

#### XRPL Handler Config

TBD

#### Solana Handler Config

TBD

## Deployment

Ensure that `ampd` is updated to the latest version that supports the gRPC interface. After updating the configuration as described above, restart `ampd` to reload the config. Once `ampd` is up and running with the gRPC server enabled, start the handler services for each chain you support:

```bash
./<chain>-handler --config-dir ~/.ampd/<chain>/
```

## Orchestration
The daemon can run without any handlers connected, but the handlers cannot run without the daemon, and will crash if unable to connect. To ensure continuous up time, both the daemon and each handler should be set to auto restart by whatever orchestration policy that is being used.
For example, when using systemd, under the `Service` section, set
```
[Service]
Restart=always
RestartSec=5s
```
The daemon and each running handler should be orchestrated as separate services.

### Checklist

Check `ampd` and handler logs to ensure they are running correctly. For each chain, monitor voting and signing activity for your verifier on Axelarscan to verify it's operating correctly.
