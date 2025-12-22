# Permission Role Management Guide

## Overview

This guide covers how to:
1. Update `ROLE_ACCESS_CONTROL` to a new multisig key
2. Register a new `ROLE_CHAIN_MANAGEMENT` (controller/Key Rotation EOA) address
3. Deregister an old controller/Key Rotation EOA address

> **⚠️ All operations require `ROLE_ACCESS_CONTROL` permission**

---

## Current Role Addresses

### Governance Multisig/Emergency Operator Multisig (ROLE_ACCESS_CONTROL)

| Network | Address |
|---------|---------|
| **Mainnet** | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |
| **Testnet** | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` |
| **Stagenet** | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` |
| **Devnet** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` ⚠️ EOA, not multisig |

### Controller/Key Rotation EOA (ROLE_CHAIN_MANAGEMENT)

| Network | Address |
|---------|---------|
| **Mainnet** | `axelar1s952p4ye4hs24hqtnwjpggl0akzpcd5uany5rw` |
| **Testnet** | `axelar1tf298zq9fn0rjlj23dmw04jfpu2whyrqsch5qn` |
| **Stagenet** | `axelar1z5fkx8jt4qthpg5dm0vwgluehuf295jgay6fs5` |
| **Devnet** | `axelar10dwjv6xj0flfj5m3ce4t5v7xpjmrt5du7h6zs6` |

---

## Prerequisites

```bash
# Set environment (mainnet, testnet, stagenet, devnet-amplifier, etc.)
ENV=${1:-mainnet}
CONFIG_FILE="./axelar-chains-config/info/$ENV.json"

# Read CHAIN_ID from config file
CHAIN_ID=$(jq -r '.axelar.chainId // empty' "$CONFIG_FILE")
if [ -z "$CHAIN_ID" ]; then
  echo "Error: Could not read chainId from $CONFIG_FILE"
  exit 1
fi

# Read NODE (RPC endpoint) from config file
NODE=$(jq -r '.axelar.rpc // empty' "$CONFIG_FILE")
if [ -z "$NODE" ]; then
  echo "Error: Could not read RPC endpoint from $CONFIG_FILE"
  exit 1
fi

# Set multisig address based on environment (see table above)
case $ENV in
  mainnet)
    MULTISIG_ADDR="axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am"
    ;;
  testnet)
    MULTISIG_ADDR="axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7"
    ;;
  stagenet)
    MULTISIG_ADDR="axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky"
    ;;
  devnet-amplifier)
    MULTISIG_ADDR="axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"
    ;;
  *)
    echo "Error: Unknown environment $ENV"
    exit 1
    ;;
esac

echo "=== Environment: $ENV ==="
echo "=== Chain ID: $CHAIN_ID ==="
echo "=== Node: $NODE ==="
echo "=== Multisig: $MULTISIG_ADDR ==="
```

---

## Multisig Transaction Process

Since `ROLE_ACCESS_CONTROL` is held by a multisig, all transactions require multi-party signing:

> **Note:** On **devnet-amplifier**, `ROLE_ACCESS_CONTROL` is held by a regular EOA (not a multisig), so you can skip the multi-sign process and broadcast transactions directly with `--from <key> -y` instead of `--generate-only`.

1. Generate unsigned transaction (`--generate-only`)
2. Each signer signs individually (`axelard tx sign`)
3. Combine signatures (`axelard tx multi-sign`)
4. Broadcast transaction (`axelard tx broadcast`)

---

## Step 1: Update Governance Key (ROLE_ACCESS_CONTROL)

Updates the `ROLE_ACCESS_CONTROL` to a new multisig key.

| Parameter | Description |
|-----------|-------------|
| `threshold` | Number of signatures required (e.g., `2` for 2-of-3 multisig) |
| `pubKey1`, `pubKey2`, ... | JSON-encoded public keys of multisig members |

### 1.1 Generate unsigned transaction

```bash
axelard tx permission update-governance-key 2 \
  '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"A1B2C3..."}' \
  '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"D4E5F6..."}' \
  '{"@type":"/cosmos.crypto.secp256k1.PubKey","key":"G7H8I9..."}' \
  --from $MULTISIG_ADDR \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.007uaxl \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --generate-only > unsigned_tx.json
```

### 1.2 Each signer signs individually

```bash
# Signer 1 (on their own machine)
axelard tx sign unsigned_tx.json \
  --from signer1 \
  --multisig $MULTISIG_ADDR \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --sign-mode amino-json \
  --output-document sig1.json

# Signer 2 (on their own machine)
axelard tx sign unsigned_tx.json \
  --from signer2 \
  --multisig $MULTISIG_ADDR \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --sign-mode amino-json \
  --output-document sig2.json
```

### 1.3 Combine signatures

```bash
axelard tx multi-sign unsigned_tx.json gov-multisig sig1.json sig2.json \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --output-document signed_tx.json
```

### 1.4 Broadcast transaction

```bash
axelard tx broadcast signed_tx.json --node $NODE
```

### Get public key from address

```bash
axelard query auth account <address> --node $NODE -o json | jq '.account.pub_key'
```

---

## Step 2: Register New Controller/Key Rotation EOA (ROLE_CHAIN_MANAGEMENT)

Registers a new address with `ROLE_CHAIN_MANAGEMENT` permission.

### 2.1 Generate unsigned transaction

```bash
axelard tx permission register-controller <new-controller-address> \
  --from $MULTISIG_ADDR \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.007uaxl \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --generate-only > unsigned_register.json
```

### 2.2-2.4 Sign, combine, and broadcast

Follow the same signing process as Step 1.2-1.4.

---

## Step 3: Deregister Old Controller/Key Rotation EOA

Removes `ROLE_CHAIN_MANAGEMENT` from an old controller/Key Rotation EOA address.

### 3.1 Generate unsigned transaction

```bash
axelard tx permission deregister-controller <old-controller-address> \
  --from $MULTISIG_ADDR \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.007uaxl \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --generate-only > unsigned_deregister.json
```

### 3.2-3.4 Sign, combine, and broadcast

Follow the same signing process as Step 1.2-1.4.

---

## Verification Commands

### Verify Governance Key (ROLE_ACCESS_CONTROL)

```bash
axelard query permission governance-key --node $NODE -o json
```

### Query Permission Module Params

```bash
axelard query permission params --node $NODE -o json
```

> **Note:** There is no CLI command to list all `ROLE_CHAIN_MANAGEMENT` accounts directly. 

---

## Notes

| Operation | Required Permission | Can Have Multiple? |
|-----------|--------------------|--------------------|
| `update-governance-key` | `ROLE_ACCESS_CONTROL` | ❌ Only one |
| `register-controller` | `ROLE_ACCESS_CONTROL` | ✅ Multiple allowed |
| `deregister-controller` | `ROLE_ACCESS_CONTROL` | - |

> **💡 Tip:** All these operations can also be executed via **governance proposal** which bypasses the ante handler permission checks. See [axelard tx gov CLI reference](https://github.com/axelarnetwork/axelar-core/blob/main/docs/cli/axelard_tx_gov.md) for details on submitting governance proposals.
