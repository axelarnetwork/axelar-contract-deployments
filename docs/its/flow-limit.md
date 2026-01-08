# Flow Limit

## Overview

Flow limits control how much value can move in/out of a chain for a specific token over epoch period. This is a per-token, per-chain rate limiting mechanism that helps manage token transfer volumes and provides protection against exploits.

**Key Concepts:**
- **Epoch**: Flow counters reset at the start of each epoch period
- **Net Flow**: `|flowOut - flowIn|` - bidirectional transfers offset each other
- **Flow Limit**: Maximum allowed net flow per epoch. Setting `flowLimit = 0` disables rate limiting.
- **Per-chain, per-token**: Each TokenManager on each chain has independent flow limits

Setting a flow limit to the minimum value (1) effectively freezes transfers for that token. Setting it to `0` removes the limit entirely.

## Routine Operations

- **Baseline Rate Limiting**: Establish standard transfer caps per token/chain for normal operations
- **Capacity Planning**: Adjust limits based on expected traffic patterns and usage forecasts
- **Gradual Scaling**: Incrementally raise or lower limits as part of operational optimization
- **Performance Monitoring**: Track flow metrics to inform capacity decisions

## Emergency Scenarios

- **Abnormal Transfer Volume**: Unusual transfer activity detected (e.g., volume > 3x average of past 5 hours)
- **Token Exploit Risk**: Specific token is at risk of being exploited, requiring immediate rate limiting
- **Security Incident**: Security team identifies active token-related security issues
- **Partner Notification**: Partner or security researcher reports token compromise
- **Chain-Specific Issues**: Critical issues on a specific chain affecting token transfers
- **Immediate Containment**: Freeze tokens (set limit to 1) to stop all transfers during active threats

## Execution

### EVM

**Required Role:** ITS Operator

**Set/Adjust Flow Limit:**
```bash
ts-node evm/its.js set-flow-limit <tokenId> <flowLimit>
```

**Token ID Format:** 32-byte hex string with `0x` prefix (66 characters total)

**Quick Freeze (sets limit to 1):**
```bash
ts-node evm/its.js freeze-tokens <tokenId1> <tokenId2> ...
```

**Quick Unfreeze (removes limit):**
```bash
ts-node evm/its.js unfreeze-tokens <tokenId1> <tokenId2> ...
```

**Note:** Use `1` to effectively freeze transfers, `0` to remove limit (unfreeze), or a specific value to rate limit.

**Examples:**

**Note:** Flow limit values must be in the token's smallest unit (wei/smallest unit). The value depends on the token's decimal places. For example, a token with 18 decimals requires `1000000000000000000` (18 zeros) to represent 1 token.

```bash
ts-node evm/its.js set-flow-limit 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef 1000000000000000000000
```

Freeze a single token:
```bash
ts-node evm/its.js freeze-tokens 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

Freeze multiple tokens:
```bash
ts-node evm/its.js freeze-tokens \
  0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
```

Unfreeze a token (remove limit):
```bash
ts-node evm/its.js unfreeze-tokens 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

### Stellar

**Required Role:** ITS Operator

**Set/Adjust Flow Limit:**
```bash
ts-node stellar/its.js set-flow-limit <tokenId> <limit>
```

**Token ID Format:** 32-byte hex string with `0x` prefix (66 characters total)

**Remove Flow Limit (unfreeze):**
```bash
ts-node stellar/its.js remove-flow-limit <tokenId>
```

**Freeze (set to 1):**
```bash
ts-node stellar/its.js set-flow-limit <tokenId> 1
```

**Examples:**

Set flow limit (example: 1,000,000 = 1 token with 6 decimals, or adjust based on token's decimal configuration):
```bash
ts-node stellar/its.js set-flow-limit 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef 1000000
```

Freeze a token:
```bash
ts-node stellar/its.js set-flow-limit 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef 1
```

Remove flow limit (unfreeze):
```bash
ts-node stellar/its.js remove-flow-limit 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

### Sui

**Required Role:** Operator (OperatorCap holder)

**Set Flow Limits (multiple tokens):**
```bash
ts-node sui/its.js set-flow-limits <token-ids> <flow-limits>
```

**Token ID Format:** Sui addresses or hex strings. The system converts them internally to the appropriate format.

**Note:** Provide parallel arrays for token IDs and flow limits. Use `1` to freeze, `0` to remove limit.

**Examples:**

Set flow limits for multiple tokens (example values - adjust based on token decimals):
```bash
ts-node sui/its.js set-flow-limits \
  0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef,0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 \
  1000000000000000000000,2000000000000000000000
```

Freeze multiple tokens:
```bash
ts-node sui/its.js set-flow-limits \
  0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef,0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 \
  1,1
```

Unfreeze tokens (remove limits):
```bash
ts-node sui/its.js set-flow-limits \
  0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  0
```

## Verification

After execution, verify the flow limit was set:

**EVM:**
```bash
ts-node evm/its.js flow-limit <tokenId>
```

**Stellar:**
```bash
ts-node stellar/its.js flow-limit <tokenId>
```

**Sui:** Check the updated flow limit via cli or block explorer.
