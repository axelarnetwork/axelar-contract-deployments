# Pause ITS

## Overview

Pause/unpause controls allow you to globally stop or resume all ITS operations on a chain. When paused, the InterchainTokenService contract stops processing interchain token transfers and related operations.

This is a system-wide control that affects all tokens on the chain, making it more severe than freezing individual tokens via flow limits.

## Routine Operations

- **Scheduled Maintenance**: Pause during planned maintenance windows with advance notice
- **Upgrade Preparation**: Pause before contract upgrades to ensure clean state transitions
- **Planned Transitions**: Pause during scheduled role transfers or operational handovers
- **Development Testing**: Pause test chains during controlled development and QA cycles

## Emergency Scenarios

- **Critical Bug**: Critical bug or vulnerability discovered in ITS contract requiring immediate shutdown
- **Global Incident**: System-wide security incident affecting all tokens on the chain
- **Active Exploit**: Active exploit detected that could affect all operations
- **Emergency Shutdown**: Need to immediately stop all ITS operations on a chain
- **Widespread Containment**: Need to contain a widespread issue affecting multiple tokens simultaneously

## Execution

### EVM

**Required Role:** ITS Owner

**Pause ITS:**
```bash
ts-node evm/its.js set-pause-status true
```

**Unpause ITS:**
```bash
ts-node evm/its.js set-pause-status false
```

**Via Governance (EVM only):**
```bash
ts-node evm/its.js set-pause-status true \
  --governance \
  --activationTime <activationTime> \
  --operatorProposal \
  --generate-only proposal.json
```

### Stellar

**Required Role:** ITS Owner

**Pause ITS:**
```bash
ts-node stellar/contract.js pause InterchainTokenService
```

**Unpause ITS:**
```bash
ts-node stellar/contract.js unpause InterchainTokenService
```

## Verification

After execution, verify pause status:

**EVM:** Check via cli or block explorer.

**Stellar:**
```bash
ts-node stellar/contract.js paused InterchainTokenService
```

**Sui:** Check via cli or block explorer.
