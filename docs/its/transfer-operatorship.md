# Transfer Operatorship

## Overview

Transfer operatorship moves the ITS operator role from the current operator to a new address. The operator role controls operational settings including flow limits, trusted chains, and pause status.

This is a critical security operation used when the operator key is compromised or during scheduled role rotations.

## Routine Operations

- **Scheduled Key Rotation**: Regular operator key rotation following security best practices and schedules
- **Planned Operational Handover**: Transferring operator responsibilities to a new team or system as part of planned transitions
- **Key Management Procedures**: Updating operator keys as part of routine key management procedures

## Emergency Scenarios

- **Operator Key Compromise**: Operator key is compromised or suspected of being compromised
- **Immediate Threat Response**: Need to quickly transfer control to a secure operator address to prevent unauthorized access
- **Active Security Incident**: Security event requires immediate revocation of current operator privileges

## Execution

### EVM

**Required Role:** ITS Operator or Owner

**Direct Execution:**
```bash
ts-node evm/its.js transferOperatorship <newOperator>
```

**Examples:**

Transfer operatorship:
```bash
ts-node evm/its.js transferOperatorship 0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb5
```

### Stellar

**Required Role:** ITS Operator or Owner

**Transfer Operatorship:**
```bash
ts-node stellar/contract.js transfer-operatorship InterchainTokenService <newOperator>
```

**Examples:**

Transfer operatorship:
```bash
ts-node stellar/contract.js transfer-operatorship InterchainTokenService GB2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3
```

### Sui

**Required Role:** Owner (OwnerCap holder) or Current Operator (OperatorCap holder)

**Note:** On Sui, operatorship is managed through OperatorCap objects. Transfer operatorship by transferring the OperatorCap object to the new operator.

```bash
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OperatorCap --recipient <newOperator>
```

**Examples:**

Transfer OperatorCap:
```bash
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OperatorCap --recipient 0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88
```

## Verification

**EVM:**
```bash
ts-node evm/its.js isOperator <newOperatorAddress>
```

**Stellar:**
```bash
ts-node stellar/contract.js operator InterchainTokenService
```

**Sui:** Verify OperatorCap ownership via CLI or block explorer.

