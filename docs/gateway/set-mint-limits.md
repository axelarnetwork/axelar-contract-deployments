# Set Mint Limits

## Overview

Set mint limits controls how much of a token can be minted on the Gateway tokens over time. This is a per-token rate limiting mechanism that helps prevent unlimited minting and provides protection against exploits.

**Key Concepts:**
- **Mint Limit**: Maximum amount of a token that can be minted. Setting `limit = 1` effectively freezes mints for that token.
- **Per-token**: Each token symbol has an independent mint limit
- **Consensus Gateway Only**: This action is only available for consensus gateways (not Amplifier gateways)

Setting a mint limit to the minimum value (1) effectively freezes mints for that token. Setting it to `0` removes the limit entirely.

## Routine Operations

- **Baseline Rate Limiting**: Establish standard mint caps per token for normal operations
- **Capacity Planning**: Adjust limits based on expected traffic patterns and usage forecasts
- **Gradual Scaling**: Incrementally raise or lower limits as part of operational optimization
- **Performance Monitoring**: Track mint metrics to inform capacity decisions

## Emergency Scenarios

- **Token Exploit Risk**: Specific token is at risk of being exploited, requiring immediate mint limiting
- **Security Incident**: Security team identifies active token-related security issues
- **Partner Notification**: Partner or security researcher reports token compromise
- **Immediate Containment**: Freeze token mints (set limit to 1) to stop all mints during active threats

## Execution

### EVM

**Required Role:** Gateway Mint Limiter (consensus gateways only)

**Direct Execution:**
```bash
ts-node evm/gateway.js --action setTokenMintLimits --symbols <symbols-json> --limits <limits-json>
```

**Via Multisig (when mint limiter is a Multisig):**
```bash
ts-node evm/multisig.js --action setTokenMintLimits --symbols <symbols-json> --limits <limits-json>
```

**Token Symbols Format:** JSON array of token symbol strings (e.g., `["AXL","axlUSDC"]`)

**Mint Limits Format:** JSON array of uint256 values in smallest units, same length and order as symbols

**Examples:**

Freeze multiple tokens:
```bash
ts-node evm/gateway.js --action setTokenMintLimits --symbols '["axlUSDC","axlUSDT","AXL"]' --limits '[1,1,1]'
```

Unfreeze tokens (remove limits):
```bash
ts-node evm/gateway.js --action setTokenMintLimits --symbols '["axlUSDC"]' --limits '[0]'
```

## Verification

Check current mint limits and minted amounts:

**EVM:**
```bash
ts-node evm/gateway.js --action mintLimit --symbol <TOKEN_SYMBOL>
```

**Example:**
```bash
ts-node evm/gateway.js --action mintLimit --symbol axlUSDC
```


