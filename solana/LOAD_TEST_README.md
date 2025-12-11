# Solana ITS Load Testing Tool

A Rust-based load testing tool for Solana Interchain Token Service (ITS) operations, integrated into the Solana CLI.

## Features

- **Parallel Transaction Execution**: Generate multiple transactions concurrently using derived keypairs
- **Mnemonic-based Keypair Derivation**: Derive multiple test accounts from a single mnemonic
- **Dual Verification System**:
  - Solana on-chain transaction finalization checks
  - Axelarscan cross-chain GMP message verification
- **Resumable Verification**: Continue verification from a specific transaction if interrupted
- **Detailed Statistics**: Track transaction success rates, timing, and throughput

## Prerequisites

- Rust toolchain installed
- Solana CLI configured
- Valid mnemonic with funded accounts
- `.env` file configured with required environment variables

## Environment Setup

Create a `.env` file in the `solana/` directory with the following variables:

```bash
ENV="devnet-amplifier"
CHAIN="solana"
CLUSTER="devnet"
MNEMONIC="your twelve or twenty four word mnemonic phrase here"
```

## Commands

### Load Test Command

Run a load test by generating multiple interchain transfers:

```bash
solana/cli load-test test \
    --destination-chain 'avalanche-fuji' \
    --token-id '95dab4cde2e0b6901340c4ed4d1484424612a84c6285e2e6b2e7d1c2bbdb3c80' \
    --destination-address '0x3E2394144C21266C44854128A2b9eCcb312150d8' \
    --transfer-amount 2 \
    --time 300 \
    --delay 10 \
    --mnemonic $MNEMONIC \
    --addresses-to-derive 10 \
    --gas-value 100000 \
    --output load-test.txt
```

**Parameters:**
- `--destination-chain`: Target blockchain for token transfers
- `--token-id`: Hex-encoded token ID (32 bytes, with or without 0x prefix)
- `--destination-address`: Recipient address on destination chain
- `--transfer-amount`: Amount of tokens to transfer (supports decimals, e.g., "0.001")
- `--time`: Duration of test in seconds
- `--delay`: Delay between transaction submissions in milliseconds (default: 10ms)
- `--mnemonic`: BIP39 mnemonic for keypair derivation (can use env var MNEMONIC)
- `--addresses-to-derive`: Number of keypairs to derive from mnemonic
- `--gas-value`: Gas value for cross-chain transfers (optional)
- `--output`: Output file path for transaction signatures (default: output/load-test.txt)

### Verify Command

Verify the results of a load test:

```bash
./cli load-test verify \
  --input-file output/load-test.txt \
  --fail-output output/load-test-fail.txt \
  --pending-output output/load-test-pending.txt \
  --success-output output/load-test-success.txt \
  --delay 100
```

**Parameters:**
- `--input-file`: File containing transaction signatures to verify
- `--fail-output`: Output file for failed transactions
- `--pending-output`: Output file for pending/incomplete transactions
- `--success-output`: Output file for successfully completed transactions
- `--delay`: Delay between verification requests in milliseconds (default: 100ms)
- `--resume-from`: Resume verification from specific transaction number (1-based index)

## Verification Process

The verification command performs a two-stage check for each transaction:

1. **Solana RPC Check**: Verifies the transaction was successfully confirmed on Solana
2. **Axelarscan GMP Check**:
   - Queries Axelarscan API with the transaction signature
   - Retrieves the cross-chain message ID
   - Verifies the message was executed on the destination chain

Transactions are classified as:
- **Success**: Both Solana and cross-chain execution confirmed
- **Failed**: Transaction error or execution failure detected
- **Pending**: Transaction not yet finalized or cross-chain message not yet executed

## Example Workflow

1. **Run a 5-minute load test:**
```bash
./cli load-test test \
  --destination-chain ethereum \
  --token-id 0xabcd... \
  --destination-address 0x1234... \
  --transfer-amount 0.001 \
  --time 300 \
  --addresses-to-derive 10
```

2. **Wait for transactions to propagate** (recommended: wait a few minutes)

3. **Verify results:**
```bash
./cli load-test verify --input-file output/load-test.txt
```

4. **If verification was interrupted, resume:**
```bash
./cli load-test verify \
  --input-file output/load-test.txt \
  --resume-from 50
```

## Output Format

### Load Test Output
Transaction signatures are written one per line:
```
5a8b2f...transaction-signature-1
7c9d3e...transaction-signature-2
...
```

### Verification Output
Each output file contains transaction signatures with optional status messages:
```
5a8b2f... : error: insufficient funds
7c9d3e... : GMP status is pending (expected executed)
```

## Performance Considerations

- **Delay Parameter**: Lower delays increase throughput but may hit rate limits
- **Derived Accounts**: More accounts = more parallelism, but each needs funding
- **Verification Delay**: Adjust based on network congestion and API rate limits

## Troubleshooting

**"Must specify --addresses-to-derive when using mnemonic"**
- Solution: Add `--addresses-to-derive <number>` parameter

**"Transaction not found or not finalized"**
- Solution: Wait longer before verifying, or transaction may have failed

**"Message ID not found in Axelarscan response"**
- Solution: Transaction may not have generated a GMP message, or Axelarscan hasn't indexed it yet

**"axelarscanApi not found in chains info"**
- Solution: Ensure chains info JSON file contains the `axelarscanApi` field under `axelar`

## Architecture

The load test tool:
- Uses BIP44 derivation path `m/44'/501'/N'` for Solana keypairs
- Implements tokio async runtime for parallel transaction execution
- Maintains task tracking to ensure all transactions complete before exit
- Streams results to disk incrementally to prevent data loss

## Integration with Existing CLI

The load test module integrates seamlessly with the existing Solana CLI:
- Uses the same configuration system (chains info, environment selection)
- Leverages existing ITS instruction builders
- Shares RPC client configuration and transaction signing logic

