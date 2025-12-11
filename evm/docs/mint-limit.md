### Mint limit management

This document explains how to manage per-token mint limits on an `AxelarGateway` using the deployment scripts.

Mint limits are enforced on each EVM chain by the gateway contract. They can be updated directly by the current `mintLimiter` or via the Multisig acting as mint limiter where configured.

---

### 1. Updating mint limits via `gateway.js`

The `gateway.js` script exposes the `setTokenMintLimits` action, which calls the `AxelarGateway.setTokenMintLimits(string[] symbols, uint256[] limits)` function. This flow is intended **only for consensus gateways** (i.e. chains where `contracts.AxelarGateway.connectionType !== 'amplifier'`).

#### Requirements
- **Caller**: The sender must be the current `mintLimiter` on that chain’s `AxelarGateway`.
- **Arguments**:
  - **`--symbols`**: JSON array of token symbols (e.g. `["AXL","axlUSDC"]`).
  - **`--limits`**: JSON array of uint256 mint limits, in smallest units, same length and order as `symbols`.

#### Example (direct mint limiter)

```bash
node evm/gateway.js \
  -e mainnet \
  -n ethereum \
  --privateKey <MINT_LIMITER_KEY> \
  --action setTokenMintLimits \
  --symbols '["AXL","axlUSDC"]' \
  --limits '[20000000000000,1000000000000]' \
  -y
```

Behavior:
- Validates `symbols` and `limits` as JSON arrays.
- Ensures lengths match and values are numeric.
- Verifies that the wallet is the current `mintLimiter` on the gateway.
- Calls `setTokenMintLimits` on the target chain’s `AxelarGateway`.

---

### 2. Updating mint limits via `multisig.js`

Where the mint limiter role is assigned to a Multisig contract, use the `multisig.js` script with the `setTokenMintLimits` action.

#### Arguments
- **`--symbols`**: JSON array of token symbols (e.g. `["AXL"]`).
- **`--limits`**: JSON array of uint256 mint limits, same length and order as `symbols`.
- Other base options: `-e/--env`, `-n/--network`, `--privateKey`, `--offline`, `--nonceOffset`, etc.

#### Example (freeze mints)

```bash
node evm/multisig.js \
  -e mainnet \
  -n ethereum \
  --privateKey ledger \
  --action setTokenMintLimits \
  --symbols '["axlUSDC","axlEUROC","AXL","axlETH","axlUSDT","axlDAI","axlWBTC"]' \
  --limits '[1,1,1,1,1,1,1]' \
  --nonceOffset <NONCE_OFFSET> \
  -y --offline
```

Behavior:
- Validates `symbols` and `limits` using the shared utilities.
- Encodes and submits a multisig proposal targeting `AxelarGateway.setTokenMintLimits`.
- In online mode, prints current token addresses and existing mint limits for visibility.

---

### 3. Notes & best practices

- **Consistency across chains**: For globally relevant tokens, ensure mint limits are updated coherently across all relevant EVM chains.
- **Monitoring**: Pair mint limit changes with monitoring/alerting so large deviations can be detected quickly.
- **Dry runs**: Prefer `--offline` mode with `multisig.js` to stage and inspect signed transactions before broadcasting.
- **Governance records**: When used in response to governance decisions, store the exact `symbols`/`limits` JSON payloads alongside the proposal for auditability.
