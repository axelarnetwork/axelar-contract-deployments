# Add Solana Handler to ampd

This guide assumes you are already running `ampd` v1.14.2 with the modular handler architecture.
If you haven't upgraded yet, follow the [ampd v1.14.2 release guide](../ampd/2025-12-19-ampd-v1.14.2.md) first.

## Step 1: Update ampd Config

Add the following entry to your `~/.ampd/config.toml` and restart `ampd`.

### Testnet

```toml
[[grpc.blockchain_service.chains]]
chain_name = "solana"
voting_verifier = "axelar19gut3kvqf57gnu5ylq474qfgk4gg5ly89cs5kk4mde688lc5adsq6qyz4h"
multisig_prover = "axelar1xdtjwhenmy80wckuntd2npd3zeqayy0q0l5dfy48g6wmu3p48pgqknnc9g"
multisig = "axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
```

### Mainnet

<!-- TODO: fill in mainnet addresses -->

```toml
[[grpc.blockchain_service.chains]]
chain_name = "solana"
voting_verifier = "TBD"
multisig_prover = "TBD"
multisig = "TBD"
```

## Step 2: Create Handler Config

```bash
mkdir -p ~/.ampd/solana
```

### Base config (`~/.ampd/solana/config.toml`)

```toml
ampd_url = "http://127.0.0.1:9090"
chain_name = "solana"
```

### Solana-specific config (`~/.ampd/solana/solana-handler-config.toml`)

#### Testnet

```toml
rpc_url = "https://api.devnet.solana.com"
domain_separator = "2ba3d447b55ebd352d941bad2be996868560897c257f063c9c3bcee72f82234f"
```

#### Mainnet

<!-- TODO: fill in mainnet values -->

```toml
rpc_url = "TBD"
domain_separator = "TBD"
```

## Step 3: Run the Solana Handler

### Option A: Binary

Download the binary from the [solana-handler v0.1.6 release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/solana-handler-v0.1.6).

```bash
solana-handler --config-dir ~/.ampd/solana/
```

### Option B: Docker

```bash
docker run -d \
  --name solana-handler \
  --restart unless-stopped \
  -v ~/.ampd/solana:/config \
  axelarnet/axelar-ampd-solana-handler:v0.1.6 \
  --config-dir /config
```

Ensure the handler can reach ampd's gRPC server. If running all containers on the same machine, use a shared Docker network or `--network host`.

## Step 4: Register Keys and Chain Support

Register an ed25519 public key (required for Solana, even if you already have an ecdsa key registered):

```bash
ampd register-public-key ed25519
```

Register chain support for Solana:

```bash
ampd register-chain-support amplifier solana
```

## Verification

1. Check handler logs to ensure it connects to ampd successfully.
2. Monitor voting and signing activity for your verifier on Axelarscan.
