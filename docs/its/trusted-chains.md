# Trusted Chains

## Overview

Trusted chains determine which chains can send and receive interchain token transfers via ITS. Adding a chain to the trusted list enables cross-chain transfers to/from that chain. Removing a chain disables all transfers to/from that chain.

This is a critical security control that allows operators to quickly isolate problematic chains or enable new chain connections.

## Routine Operations

- **New Chain Integration**: Enable cross-chain transfers when new chains are officially integrated
- **Planned Configuration Updates**: Refresh trusted lists as part of scheduled maintenance
- **Operational Expansion**: Add chains based on planned operational requirements
- **Development Testing**: Enable test chains for controlled development and QA environments

## Emergency Scenarios

- **Chain Compromise**: Partner chain is compromised or suspected of being compromised
- **Active Security Incident**: Active security issues reported on a specific chain
- **Critical Chain Downtime**: Chain experiencing critical downtime or instability affecting operations
- **Immediate Isolation**: Need to quickly isolate a problematic chain from the network
- **Threat Containment**: Remove chains to prevent spread of security threats or exploits

## Execution

### EVM

**Required Role:** ITS Owner or Operator

**Set Trusted Chains (add/replace):**
```bash
ts-node evm/its.js set-trusted-chains <chain1> <chain2> ...
```

**Remove Trusted Chains:**
```bash
ts-node evm/its.js remove-trusted-chains <chain1> <chain2> ...
```

**Via Governance (EVM only):**
```bash
ts-node evm/its.js set-trusted-chains <chain1> <chain2> ... \
  --governance \
  --activationTime <activationTime> \
  --generate-only proposal.json

ts-node evm/its.js remove-trusted-chains <chain1> <chain2> ... \
  --governance \
  --activationTime <activationTime> \
  --operatorProposal \
  --generate-only proposal.json
```

**Examples:**

Add a single chain:
```bash
ts-node evm/its.js set-trusted-chains polygon
```

Remove multiple chains:
```bash
ts-node evm/its.js remove-trusted-chains polygon avalanche fantom
```

### Stellar

**Required Role:** ITS Operator

**Add Trusted Chains:**
```bash
ts-node stellar/its.js add-trusted-chains <chain1> <chain2> ...
```

**Remove Trusted Chains:**
```bash
ts-node stellar/its.js remove-trusted-chains <chain1> <chain2> ...
```

**Examples:**

Add multiple chains:
```bash
ts-node stellar/its.js add-trusted-chains polygon avalanche fantom
```

Remove a single chain:
```bash
ts-node stellar/its.js remove-trusted-chains polygon
```

### Sui

**Required Role:** Owner (OwnerCap holder)

**Add Trusted Chains:**
```bash
ts-node sui/its.js add-trusted-chains <chain1> <chain2> ...
```

**Remove Trusted Chains:**
```bash
ts-node sui/its.js remove-trusted-chains <chain1> <chain2> ...
```

**Note:** On Sui, you can use special tags like `all` to target all InterchainTokenService-deployed chains.

**Examples:**

Add multiple chains:
```bash
ts-node sui/its.js add-trusted-chains polygon avalanche fantom
```

Add all chains:
```bash
ts-node sui/its.js add-trusted-chains all
```

## Verification

After execution, verify chains are added/removed:

**EVM:**
```bash
ts-node evm/its.js is-trusted-chain <chain>
```

**Stellar:**
```bash
ts-node stellar/its.js is-trusted-chain <chain>
```

**Sui:** Check via CLI or block explorer.
