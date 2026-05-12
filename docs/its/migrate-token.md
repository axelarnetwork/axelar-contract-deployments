# Migrate Interchain Token

## Overview

The migrate interchain token operation transfers the minter role from ITS to the token manager for native interchain tokens. This operation is specifically for tokens that were deployed through ITS (native interchain tokens), not custom tokens that were linked.

**Key Concepts:**
- **Native Interchain Tokens**: Tokens deployed through ITS using `deployInterchainToken`
- **Minter Role Migration**: Transfers mintership from ITS to the token manager
- **EVM Only**: This operation is only available on EVM chains

The migration process transfers the minter role to the token manager, allowing the token manager to directly control minting operations for the token.


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

After execution, verify the minter role was transferred:

**EVM:**
```bash
# Get the token manager address
ts-node evm/its.js token-manager-address <tokenId>

# Verify the token manager is now the minter (check token contract directly)
# The token manager should be the minter, not ITS
```
