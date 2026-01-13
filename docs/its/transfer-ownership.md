# Transfer Ownership

## Overview

Transfer ownership moves the ITS owner role from the current owner to a new address. The owner role has full administrative control including the ability to transfer ownership, upgrade contracts, and perform critical operations.

This is a critical security operation used when the owner key is compromised or during governance transitions.

## Routine Operations

- **Planned Governance Transitions**: Transferring ownership to governance contracts as part of scheduled governance upgrades
- **Planned Operational Handover**: Transferring ownership to a new team or system as part of planned transitions
- **Scheduled Key Management**: Updating owner keys as part of routine key management procedures

## Emergency Scenarios

- **Owner Key Compromise**: Owner key is compromised or suspected of being compromised
- **Immediate Threat Response**: Need to quickly transfer control to a secure owner address to prevent unauthorized access
- **Critical Security Incident**: Critical security event requires immediate change of contract ownership
- **Emergency Governance Migration**: Urgent transfer of ownership to governance contracts (e.g., AxelarServiceGovernance) during security incidents

## Execution

### EVM

**Required Role:** Current ITS Owner

**Direct Execution:**
```bash
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner <newOwner>
```

**Via Governance (EVM only):**
```bash
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner <newOwner> \
  --governance \
  --activationTime <activationTime> \
  --generate-only proposal.json
```

**Examples:**

Transfer ownership:
```bash
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb5
```

Transfer to governance contract:
```bash
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

### Stellar

**Required Role:** ITS Owner

**Transfer Ownership:**
```bash
ts-node stellar/contract.js transfer-ownership InterchainTokenService <newOwner>
```

**Examples:**

Transfer ownership:
```bash
ts-node stellar/contract.js transfer-ownership InterchainTokenService GB2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3
```

### Sui

**Required Role:** Owner (OwnerCap holder)

**Note:** On Sui, ownership is managed through OwnerCap objects. Transfer ownership by transferring the OwnerCap object to the new owner.

```bash
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OwnerCap --recipient <newOwner>
```

**Examples:**

Transfer OwnerCap:
```bash
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OwnerCap --recipient 0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88
```

## Verification

After execution, verify the new owner:

**EVM:**
```bash
ts-node evm/ownership.js -c InterchainTokenService --action owner
```

**Stellar:**
```bash
ts-node stellar/contract.js owner InterchainTokenService
```

**Sui:** Verify OwnerCap ownership via CLI or block explorer.

