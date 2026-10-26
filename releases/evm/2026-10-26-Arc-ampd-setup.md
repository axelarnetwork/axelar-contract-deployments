# Add Arc Handler to ampd

This guide assumes you are already running `ampd` v1.14.2 with the modular handler architecture.
If you haven't upgraded yet, follow the [ampd v1.14.2 release guide](../ampd/2025-12-19-ampd-v1.14.2.md) first.

Arc is an EVM chain and uses the shared EVM handler. If you already run the EVM handler for another chain (e.g. `flow`, `xrpl-evm`, `monad`), this is one more instance of the same binary/image with its own config directory.

## Step 1: Update ampd Config

Add the following entry to your `~/.ampd/config.toml` and restart `ampd`.

### Mainnet

```toml
[[grpc.blockchain_service.chains]]
chain_name = "arc"
voting_verifier = "axelar1ex6c4hxf8xmxz35xqdq02ftwp6xngwhfddjdyj77adnewv7p8pws0l6x8p"
multisig_prover = "axelar1y7es3yxqq9ryex9nytkjk4hl4mh727kmxwk7kp07jscw50p6htjqkt8hfg"
multisig = "axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5"
```

## Step 2: Create Handler Config

```bash
mkdir -p ~/.ampd/arc
```

### Base config (`~/.ampd/arc/config.toml`)

```toml
ampd_url = "http://127.0.0.1:9090"
chain_name = "arc"
```

### EVM-specific config (`~/.ampd/arc/evm-handler-config.toml`)

#### Mainnet

```toml
rpc_url = "your-arc-mainnet-rpc-url"
finalization = "RPCFinalizedBlock"
```

## Step 3: Run the Arc Handler

### Option A: Binary

Download the binary from the [evm-handler v0.1.6 release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/evm-handler-v0.1.6) (any `evm-handler` release from v0.1.1 onwards supports Arc).

```bash
evm-handler --config-dir ~/.ampd/arc/
```

### Option B: Docker

```bash
docker run -d \
  --name arc-handler \
  --restart unless-stopped \
  -v ~/.ampd/arc:/config \
  axelarnet/axelar-ampd-evm-handler:v0.1.6 \
  --config-dir /config
```

Ensure the handler can reach ampd's gRPC server. If running all containers on the same machine, use a shared Docker network or `--network host`.

## Step 4: Register Keys and Chain Support

Arc uses ecdsa signatures. If you already have an ecdsa public key registered (you do if you sign for any other EVM chain), skip this step; otherwise:

```bash
ampd register-public-key ecdsa
```

Register chain support for Arc:

```bash
ampd register-chain-support amplifier arc
```

## Verification

1. Check handler logs to ensure it connects to ampd successfully. The EVM handler queries the finalized block from your RPC on startup and exits if that call fails.
2. Confirm your verifier lists `arc` under `supported_chains`:

   ```bash
   axelard q wasm contract-state smart axelar1rpj2jjrv3vpugx9ake9kgk3s2kgwt0y60wtkmcgfml5m3et0mrls6nct9m \
     '{"verifier":{"service_name":"amplifier","verifier":"<your-verifier-address>"}}' \
     --node https://axelar-rpc.polkachu.com
   ```

3. Monitor voting and signing activity for your verifier on Axelarscan once the Arc gateway is live.
