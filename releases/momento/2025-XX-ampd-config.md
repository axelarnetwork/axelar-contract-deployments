# Momento Ampd Configuration v1.X.X

|                | **Owner**                                 |
| -------------- | ----------------------------------------- |
| **Created By** | @[github-username] <email@duetchebank.com> |
| **Deployment** | @[github-username] <email@duetchebank.com> |

| **Network**          | **Deployment Status** | **Date** |
| -------------------- | --------------------- | -------- |
| **Devnet Amplifier** | -                     | TBD      |
| **Stagenet**         | -                     | TBD      |
| **Testnet**          | -                     | TBD      |
| **Mainnet**          | -                     | TBD      |

[Release](https://github.com/axelarnetwork/axelar-amplifier/releases/tag/ampd-vX.X.X)

## Background

This configuration update adds Momento chain support to the Axelar Amplifier daemon (ampd), enabling validators to participate in message verification and signing for the Momento chain integration with Duetche Bank.

## Prerequisites

- [Momento GMP](./2025-XX-GMP-v6.X.X.md) deployed
- [Momento ITS](./2025-XX-ITS-v2.X.X.md) deployed (if applicable)
- Verifier and Prover contracts deployed on Axelar

## Verifier and Prover Deployment

### 1. Deploy Voting Verifier

```bash
# Set environment
export ENV=<devnet-amplifier|stagenet|testnet|mainnet>
export CHAIN=momento

# Deploy voting verifier contract
ts-node cosmwasm/deploy-contract.js \
    --contract VotingVerifier \
    --chain momento \
    --version v1.0.0 \
    --env $ENV
```

### 2. Deploy Multisig Prover

```bash
# Deploy multisig prover contract
ts-node cosmwasm/deploy-contract.js \
    --contract MultisigProver \
    --chain momento \
    --version v1.0.0 \
    --env $ENV
```

### 3. Register Contracts with Router

```bash
# Register verifier with router
ts-node cosmwasm/submit-proposal \
    router-register-verifier momento [VERIFIER_ADDRESS] \
    -t "Register Momento verifier" \
    -d "Register voting verifier for Momento chain" \
    --deposit 100000000uaxl

# Register prover with router
ts-node cosmwasm/submit-proposal \
    router-register-prover momento [PROVER_ADDRESS] \
    -t "Register Momento prover" \
    -d "Register multisig prover for Momento chain" \
    --deposit 100000000uaxl
```

## Ampd Configuration

Add the following configuration to your ampd `config.toml` file:

### Basic Configuration

```toml
# Add to the basic config section if not already present
[tm_grpc_timeout]
nanos = 0
secs = 5

[tofnd_config.timeout]
nanos = 0
secs = 3

[grpc]
ip_addr = '127.0.0.1'
port = 9091  # Use different port if 9090 is taken by axelard
global_concurrency_limit = 1024
concurrency_limit_per_connection = 32
request_timeout = '30s'
```

### Momento Chain Handlers

#### Devnet Amplifier

```toml
[[handlers]]
chain_name = "momento"
chain_rpc_url = "[MOMENTO_TESTNET_RPC_URL]"
cosmwasm_contract = "[VERIFIER_CONTRACT_ADDRESS]"
type = "EvmMsgVerifier"

[[handlers]]
chain_name = "momento"
cosmwasm_contract = "[PROVER_CONTRACT_ADDRESS]"
type = "EvmMultisigProver"
```

#### Testnet

```toml
[[handlers]]
chain_name = "momento"
chain_rpc_url = "[MOMENTO_TESTNET_RPC_URL]"
cosmwasm_contract = "[VERIFIER_CONTRACT_ADDRESS]"
type = "EvmMsgVerifier"
finality = "finalized"  # or confirmation-based finality
confirmation_height = 6

[[handlers]]
chain_name = "momento"
cosmwasm_contract = "[PROVER_CONTRACT_ADDRESS]"
type = "EvmMultisigProver"
```

#### Mainnet

```toml
[[handlers]]
chain_name = "momento"
chain_rpc_url = "[MOMENTO_MAINNET_RPC_URL]"
cosmwasm_contract = "[VERIFIER_CONTRACT_ADDRESS]"
type = "EvmMsgVerifier"
finality = "finalized"
confirmation_height = 12

[[handlers]]
chain_name = "momento"
cosmwasm_contract = "[PROVER_CONTRACT_ADDRESS]"
type = "EvmMultisigProver"
```

### Advanced Configuration Options

For chains with specific requirements, additional configuration may be needed:

```toml
[[handlers]]
chain_name = "momento"
chain_rpc_url = "[RPC_URL]"
cosmwasm_contract = "[CONTRACT_ADDRESS]"
type = "EvmMsgVerifier"

# Optional configurations
finality = "finalized"  # Options: "finalized", "safe", "latest"
confirmation_height = 12  # Number of confirmations required
block_batch_size = 100  # Number of blocks to process in batch
message_batch_size = 50  # Number of messages to verify in batch
retry_count = 3  # Number of retries for RPC calls
retry_delay_ms = 1000  # Delay between retries in milliseconds

# Rate limiting (if needed)
[handlers.rate_limit]
requests_per_second = 10
burst_size = 20
```

## Validator Setup

### 1. Update Ampd Binary

```bash
# Download latest ampd binary
wget https://github.com/axelarnetwork/axelar-amplifier/releases/download/ampd-vX.X.X/ampd-linux-amd64
chmod +x ampd-linux-amd64
sudo mv ampd-linux-amd64 /usr/local/bin/ampd

# Verify version
ampd --version
```

### 2. Update Configuration

```bash
# Backup existing config
cp ~/.ampd/config.toml ~/.ampd/config.toml.backup

# Add Momento configuration to config.toml
# (Add the handlers configuration from above)

# Restart ampd service
sudo systemctl restart ampd
```

### 3. Verify Configuration

```bash
# Check ampd logs
journalctl -u ampd -f

# Look for successful connection to Momento RPC
# Should see: "Successfully connected to momento chain"
```

## Worker Set Management

### 1. Register Workers

For validators to participate in Momento verification:

```bash
# Register worker for Momento chain
ts-node cosmwasm/worker-set register \
    --chain momento \
    --worker [WORKER_ADDRESS] \
    --env $ENV
```

### 2. Authorize Workers

```bash
# Authorize workers for message verification
ts-node cosmwasm/worker-set authorize \
    --chain momento \
    --workers [WORKER_ADDRESSES] \
    --env $ENV
```

### 3. Update Worker Set

When validator set changes:

```bash
# Update worker set
ts-node cosmwasm/submit-proposal \
    update-worker-set momento \
    -t "Update Momento worker set" \
    -d "Update validator set for Momento chain" \
    --deposit 100000000uaxl
```

## Monitoring and Health Checks

### 1. Ampd Health Check

```bash
# Check ampd status
curl http://localhost:9091/health

# Expected response:
# {"status": "healthy", "chains": ["momento", ...]}
```

### 2. Chain Connection Status

```bash
# Check Momento RPC connection
curl -X POST [MOMENTO_RPC_URL] \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

### 3. Message Verification Status

```bash
# Query verifier contract for recent verifications
axelard query wasm contract-state smart [VERIFIER_ADDRESS] \
  '{"get_verification_status": {"message_id": "[MESSAGE_ID]"}}'
```

## Troubleshooting

### Common Issues

1. **"Failed to connect to Momento RPC"**
   - Verify RPC URL is correct and accessible
   - Check firewall rules
   - Ensure RPC endpoint supports required methods

2. **"Contract not found" error**
   - Verify verifier/prover addresses are correct
   - Ensure contracts are deployed on correct network

3. **"Insufficient gas" for verification**
   - Check ampd wallet balance
   - Adjust gas prices in configuration

### Debug Commands

```bash
# Check ampd logs for errors
journalctl -u ampd --since "1 hour ago" | grep -i error

# Test RPC connection
curl -X POST [MOMENTO_RPC_URL] \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"net_version","params":[],"id":1}'

# Query contract info
axelard query wasm contract [CONTRACT_ADDRESS]
```

## Rollback Procedure

If issues arise with the Momento integration:

1. **Stop ampd service**
   ```bash
   sudo systemctl stop ampd
   ```

2. **Restore previous configuration**
   ```bash
   cp ~/.ampd/config.toml.backup ~/.ampd/config.toml
   ```

3. **Restart with previous version**
   ```bash
   sudo systemctl start ampd
   ```

## Checklist

### Pre-Deployment
- [ ] Verifier contract deployed
- [ ] Prover contract deployed
- [ ] Contracts registered with router
- [ ] RPC endpoints tested

### Configuration
- [ ] Ampd config updated with Momento handlers
- [ ] Basic timeout settings configured
- [ ] gRPC port configured (if needed)
- [ ] RPC URL verified

### Validation
- [ ] Ampd successfully connects to Momento RPC
- [ ] Message verification working
- [ ] Proof generation successful
- [ ] Health checks passing

### Post-Deployment
- [ ] Monitoring alerts configured
- [ ] Documentation updated
- [ ] Validator operators notified
- [ ] Support channels updated

## Contract Addresses

| Contract | Devnet | Testnet | Mainnet |
|----------|--------|---------|---------|
| Voting Verifier | axelar1... | axelar1... | axelar1... |
| Multisig Prover | axelar1... | axelar1... | axelar1... |
| Router | axelar1... | axelar1... | axelar1... |

## Support

For technical support:
- Duetche Bank Team: [technical-team@duetchebank.com]
- Axelar Validator Support: [validators@axelar.network]
- Ampd Documentation: https://docs.axelar.dev/validator/ampd

## Notes

- Ensure ampd version is compatible with Momento integration
- Monitor RPC rate limits to avoid throttling
- Keep configuration synchronized across validator infrastructure
- Document any custom modifications to standard configuration