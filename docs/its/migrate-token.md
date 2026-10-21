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

## Batch migration

Use `migrate-interchain-tokens <token-ids...>` to migrate multiple tokens on each selected chain:

```bash
ts-node evm/its.js migrate-interchain-tokens <tokenId1> <tokenId2> --yes
```

Accepts 1–300 unique token IDs and executes one atomic ITS multicall per selected chain. More than 300 IDs are rejected. A failed migration reverts the entire multicall.

The command simulates the multicall and estimates gas before sending or generating a proposal. Direct execution uses the estimate plus 20% as its gas limit, replacing any configured gas limit while preserving fee settings. The same governance flags as single-token migration are supported; governance simulation uses the configured governance contract as the caller. It does not test proposal approval, delivery, or governance execution overhead. Simulation reflects current state and does not guarantee later execution.

To submit an operator proposal, set the Axelar proposer's `MNEMONIC` locally and run:

```bash
ts-node evm/its.js migrate-interchain-tokens <tokenId1> <tokenId2> --governance --operatorProposal
```

The command displays the proposal and proposer, then asks for confirmation before submitting through `AxelarnetGateway`. Omit `--operatorProposal` for a timelock proposal. Add `--generate-only proposal.json` to save a preview without submitting. After passage and delivery, execute the approved call on the destination governance contract separately.
