## Interchain Token Service Rate Limits

### Overview

The Interchain Token Service (ITS) supports **per-token flow limits** to rate-limit how much value can move in or out of a chain over a period of time.  
These limits are managed by the ITS operator on each chain via the `evm/its.js` script.


#### Key Concepts

- **Epoch**: 6 hours (hardcoded). Flow counters reset at the start of each epoch.
- **Net Flow**: `|flowOut - flowIn|` - bidirectional transfers offset each other
- **Flow Limit**: Maximum allowed net flow per epoch. Setting `flowLimit = 0` disables rate limiting.
- **Per-chain, per-token**: Each TokenManager on each chain has independent flow limits
- **NOT per-chain-pair**: destination chains or source chains interacting with a specific chain share same flow limit for a given token
- Flow limits protect against exploits by capping potential losses per epoch

#### Example Flow Tracking

```
Epoch starts, flowLimit = 10,000 tokens
T+1h: Send 8,000 OUT    → netFlow = 8,000   ✅
T+2h: Receive 5,000 IN  → netFlow = 3,000   ✅
T+3h: Send 8,000 OUT    → netFlow = 11,000  ❌ REVERTS (FlowLimitExceeded)
T+6h: New epoch         → netFlow = 0       (counters reset)
```

All examples below assume:

```bash
ts-node evm/its.js <command>
```

where:
- `-e, --env` is the deployment environment (e.g. `mainnet`, `testnet`).
- `-n, --chain` is the EVM chain name from the Axelar chains config.
- `-p, --privateKey` is set

---

### Prerequisites

- ITS must be deployed and configured for the target chain.
- You have a funded EVM wallet with:
  - Access to the private key via `.env` (see main `README.md`).
  - **Operator** privileges on the ITS contract for **write** operations (`set-flow-limit`, `freeze-tokens`, `unfreeze-tokens`).

You can use the standard EVM options for `its.js`, for example:

```bash
ts-node evm/its.js flow-limit <token-id>
```

---

### Commands

#### 1. Get Flow Limit

Query the current **flow limit** for a token on the current chain.

```bash
ts-node evm/its.js flow-limit <token-id>
```

- **`token-id`**: ITS token identifier (32‑byte hex string, with or without `0x` prefix).

This prints the raw on-chain value of the flow limit for that token on the current chain.

---

#### 2. Get Flow In/Out Amounts

Inspect how much value has flowed **into** or **out of** the chain for a given token.

```bash
# Flow out amount (leaving current chain)
ts-node evm/its.js flow-out-amount <token-id>

# Flow in amount (arriving to current chain)
ts-node evm/its.js flow-in-amount <token-id>
```

Use these to monitor how close a token is to hitting its rate limit.

---

#### 3. Set Flow Limit (Single Token)

Set or update the **flow limit** for a single token.

```bash
ts-node evm/its.js set-flow-limit <token-id> <flow-limit>
```

- **`token-id`**: ITS token identifier.
- **`flow-limit`**: New flow limit for this token on the current chain.

Notes:
- Requires **ITS operator** privileges.
- `flow-limit = 0` effectively **removes** the limit (no rate limiting for that token on this chain).

---

#### 4. Freeze Tokens on a Chain

Freeze one or more ITS tokens on the current chain by setting their flow limits to the minimum value (1).

```bash
ts-node evm/its.js freeze-tokens <token-id-1> <token-id-2> ...
```

Behavior:
- Internally calls `setFlowLimits(tokenIds, flowLimits)` with each `flowLimit` set to `1`.
- Effectively **halts outbound flows** for those token IDs on the current chain (subject to implementation details).

Use this for emergency response when you need to stop transfers for specific tokens.

---

#### 5. Unfreeze Tokens on a Chain

Unfreeze one or more ITS tokens on the current chain by setting their flow limits to `0`.

```bash
ts-node evm/its.js unfreeze-tokens <token-id-1> <token-id-2> ...
```

Behavior:
- Internally calls `setFlowLimits(tokenIds, flowLimits)` with each `flowLimit` set to `0`.
- **Re-enables transfers** for those tokens on the current chain by removing the per-chain flow limit.

---

### Common Workflows

#### Safely Increasing a Flow Limit

1. **Check current flow metrics**
   ```bash
   ts-node evm/its.js flow-limit <token-id>
   ts-node evm/its.js flow-out-amount <token-id>
   ```
2. **Decide on new limit** (based on business/operational constraints).
3. **Apply new limit**
   ```bash
   ts-node evm/its.js set-flow-limit <token-id> <new-flow-limit>
   ```

#### Emergency Freeze and Later Unfreeze

1. **Freeze tokens** on the affected chain:
   ```bash
   ts-node evm/its.js freeze-tokens <token-id-1> <token-id-2>
   ```
2. Investigate and communicate with stakeholders.
3. **Unfreeze tokens** once it is safe:
   ```bash
   ts-node evm/its.js unfreeze-tokens <token-id-1> <token-id-2>
   ```

---

### Troubleshooting

| Error / Symptom | Likely Cause | Suggested Fix |
|-----------------|-------------|---------------|
| `TokenManager for tokenId ... does not yet exist.` | Token not yet deployed/linked on this chain. | Verify ITS configuration and that the token was deployed/linked on this chain. |
| `flow-limit` / `flow-in-amount` / `flow-out-amount` return 0 unexpectedly | Token has not yet been used or limits were never set. | Send a small transfer or set a non‑zero limit where appropriate. |
| `set-flow-limit` / `freeze-tokens` / `unfreeze-tokens` revert | Caller is not the ITS operator. | Ensure you are using the ITS operator wallet for the target chain. |


