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

#### Mainnet example

```toml
[grpc]
concurrency_limit_per_connection="32"
global_concurrency_limit="1024"
ip_addr="127.0.0.1"
port="9090"
request_timeout="30s"

[[grpc.blockchain_service.chains]]
chain_name="flow"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0"
voting_verifier="axelar1kkqdsqvwq9a7p9fj0w89wpx2m2t0vrxl782aslhq0kdw2xxd2aesv3un04"

[[grpc.blockchain_service.chains]]
chain_name="sui"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6"
voting_verifier="axelar1sykyha8kzf35kc5hplqk76kdufntjn6w45ntwlevwxp74dqr3rvsq7fazh"

[[grpc.blockchain_service.chains]]
chain_name="stellar"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1wdgp5xyqjyv5zsq86n6pah2lsmd46mn0gt4055mvvk6mezn9skqs6p93dg"
voting_verifier="axelar1dalnx2yvmu3g3aau8m7fj426fk9u8dnzlr5azvqmr4x82rtclats8lhjmu"

[[grpc.blockchain_service.chains]]
chain_name="xrpl-evm"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar198xehj5htckk75s8wcamxerxtdc45669zdqjmr69guveqntj9f6s5rqq55"
voting_verifier="axelar1q8kn9t39ddpce42atk0d6wpdudr6djqxmz689m3nxy92ck0nnftqxfsuyk"

[[grpc.blockchain_service.chains]]
chain_name="xrpl"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar15mhhuf887t6nfx2t0vuc6kx9w2uk65h939awmz6n7r6ggzyf659st25hff"
voting_verifier="axelar14rd4uyrqyl0tw75gjn8zqfppmy08t3x3wrsujeqp37l0hghduanscfvkz6"

[[grpc.blockchain_service.chains]]
chain_name="plume"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1ll4yhqtldlgqwqthyffqln3cyr2f8ydzhv0djpjyp6sk4v5k4kqqrs60s7"
voting_verifier="axelar1nrdqke6tcxjuymg5gyd9x3yg35n3wrgarnj3sqskp98z2xnvlx9q82f63t"

[[grpc.blockchain_service.chains]]
chain_name="hedera"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1e7z2faehrvpwl3apq3srr8djp386urvm2fgw3yafmju6slphhe8skecrwk"
voting_verifier="axelar1q8q8qq59yv9wmhcreu5ykt4azsk83ttve4e7jyavt32k6jq862xsqexnfh"

[[grpc.blockchain_service.chains]]
chain_name="berachain"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1k483q898t5w0acqzxhdjlsmnpgcxxa49ye8m46757n8mtk70ugtsu927xw"
voting_verifier="axelar1xx6xdw6mwmfl6u2jygq0zfx2q6uyc8emtt29j9wg78l2l4p739nqmwsgal"

[[grpc.blockchain_service.chains]]
chain_name="hyperliquid"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1fxd8rq5j6wluyc07vl9vqr4xmdxxm25l2gd6m2an20mn5fdnzy6qll2nxx"
voting_verifier="axelar1n64vk7l3zagh2eadkuhl7602lxluu86dn9smfxyp7c2e4v8pqj5sv4ypjr"

[[grpc.blockchain_service.chains]]
chain_name="monad"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1dt6apz0m2lkuls3ah2h7zw277r0v50668fxytqtdxv83yzs4n69qlutnpk"
voting_verifier="axelar1u080xgqqu9zhl4e9hf2ktdny4pq6kc2pmh6u6mlv8nw5zjvcetvqqjzeu0"
```

#### Testnet example

```toml
[grpc]
concurrency_limit_per_connection="32"
global_concurrency_limit="1024"
ip_addr="127.0.0.1"
port="9090"
request_timeout="30s"

[[grpc.blockchain_service.chains]]
chain_name="flow"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0"
voting_verifier="axelar1kkqdsqvwq9a7p9fj0w89wpx2m2t0vrxl782aslhq0kdw2xxd2aesv3un04"

[[grpc.blockchain_service.chains]]
chain_name="hedera"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1kleasry5ed73a8u4q6tdeu80hquy4nplfnrntx3n6agm2tcx40fssjk7gj"
voting_verifier="axelar1ce9rcvw8htpwukc048z9kqmyk5zz52d5a7zqn9xlq2pg0mxul9mqxlx2cq"

[[grpc.blockchain_service.chains]]
chain_name="sui"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1v8jrupu2rqpskwgtr69max0ajul92q8z5mdxd505m2hu3xc5jzcqm8zyc6"
voting_verifier="axelar1sykyha8kzf35kc5hplqk76kdufntjn6w45ntwlevwxp74dqr3rvsq7fazh"

[[grpc.blockchain_service.chains]]
chain_name="xrpl-evm"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar198xehj5htckk75s8wcamxerxtdc45669zdqjmr69guveqntj9f6s5rqq55"
voting_verifier="axelar1q8kn9t39ddpce42atk0d6wpdudr6djqxmz689m3nxy92ck0nnftqxfsuyk"

[[grpc.blockchain_service.chains]]
chain_name="xrpl"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1k82qfzu3l6rvc7twlp9lpwsnav507czl6xyrk0xv287t4439ymvsl6n470"
voting_verifier="axelar1pnynr6wnmchutkv6490mdqqxkz54fnrtmq8krqhvglhsqhmu7wzsnc86sy"

[[grpc.blockchain_service.chains]]
chain_name="plume"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1ll4yhqtldlgqwqthyffqln3cyr2f8ydzhv0djpjyp6sk4v5k4kqqrs60s7"
voting_verifier="axelar1nrdqke6tcxjuymg5gyd9x3yg35n3wrgarnj3sqskp98z2xnvlx9q82f63t"

[[grpc.blockchain_service.chains]]
chain_name="monad"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1dt6apz0m2lkuls3ah2h7zw277r0v50668fxytqtdxv83yzs4n69qlutnpk"
voting_verifier="axelar1u080xgqqu9zhl4e9hf2ktdny4pq6kc2pmh6u6mlv8nw5zjvcetvqqjzeu0"

[[grpc.blockchain_service.chains]]
chain_name="berachain"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1k483q898t5w0acqzxhdjlsmnpgcxxa49ye8m46757n8mtk70ugtsu927xw"
voting_verifier="axelar1xx6xdw6mwmfl6u2jygq0zfx2q6uyc8emtt29j9wg78l2l4p739nqmwsgal"

[[grpc.blockchain_service.chains]]
chain_name="hyperliquid"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1fxd8rq5j6wluyc07vl9vqr4xmdxxm25l2gd6m2an20mn5fdnzy6qll2nxx"
voting_verifier="axelar1n64vk7l3zagh2eadkuhl7602lxluu86dn9smfxyp7c2e4v8pqj5sv4ypjr"

[[grpc.blockchain_service.chains]]
chain_name="celo-sepolia"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar1huplwem2amlects7n06llvl46m5tfm33vtty6p82d7lmka5kmmhsrdywf4"
voting_verifier="axelar1ccyfamfvzvheec5c4knvq0l5g42knemfrnaq6t6znuwskt963k5smr9aam"

[[grpc.blockchain_service.chains]]
chain_name="memento-demo"
multisig="axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
multisig_prover="axelar13s3xyvcjpetwdfyc9q2hh9nc3hdvf7cvtkh33qk0g8adjjjgrk6qeacv67"
voting_verifier="axelar1uekdelqqxxuq5e6jxttlaxrhq3aq2ksn45h9lvtljc6hayeqe95qss5s6v"
```

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
├── flow/
│   ├── config.toml                 # Base config for EVM handler
│   └── evm-handler-config.toml     # flow EVM-specific config
├── xrpl-evm/
│   ├── config.toml                 # Base config for EVM handler
│   └── evm-handler-config.toml     # xrpl-evm EVM-specific config
├── sui/
│   ├── config.toml                 # Base config for Sui handler
│   └── sui-handler-config.toml     # Sui-specific config
├── stellar/
│   ├── config.toml                 # Base config for Stellar handler
│   └── stellar-handler-config.toml # Stellar-specific config
└── xrpl/
    ├── config.toml                 # Base config for XRPL handler (when available)
    └── xrpl-handler-config.toml    # XRPL-specific config (when available)
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

Create a file named `xrpl-handler-config.toml` in your configuration directory:

```toml
rpc_url = "[YOUR_XRPL_CHAIN_RPC_URL]"
```

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
