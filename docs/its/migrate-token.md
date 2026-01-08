# Migrate Interchain Token

## Overview

Token migration allows you to move an interchain token to a new token manager contract while preserving its interchain functionality. This is useful when a token contract needs to be upgraded or replaced, or when migrating to a more secure implementation.

The migration process moves the token's state and functionality to a new contract, ensuring continuity of interchain transfers.

## Routine Operations

Token migration is used for planned operational activities:
- **Planned Contract Upgrades**: Move tokens to improved implementations as part of scheduled upgrades
- **Token Standard Adoption**: Adopt updated token standards through planned migration processes
- **Feature Enhancements**: Migrate to managers with better features as part of operational improvements
- **Migration Testing**: Practice migrations on testnet before mainnet deployment

## Emergency Scenarios

Token migration is critical for emergency response:
- **Token Compromise**: Token contract has been compromised and requires immediate migration to a secure contract
- **Active Security Incident**: Active security issues require urgent migration to a secure contract
- **Critical Vulnerability**: Critical bugs in token contract require emergency migration to patched version
- **Incident Recovery**: Recovering from a compromised or vulnerable token implementation

## Execution

### EVM

**Required Role:** ITS Owner

**Direct Execution:**
```bash
ts-node evm/its.js migrate-interchain-token <tokenId>
```

**Token ID Format:** 32-byte hex string with `0x` prefix (66 characters total)

**Via Governance (EVM only):**
```bash
ts-node evm/its.js migrate-interchain-token <tokenId> \
  --governance \
  --activationTime <activationTime> \
  --operatorProposal \
  --generate-only proposal.json
```

**Examples:**

Migrate token (direct execution):
```bash
ts-node evm/its.js migrate-interchain-token 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
```

## Verification

After execution, verify migration was successful:

**EVM:**
```bash
ts-node evm/its.js token-manager-address <tokenId>
```
