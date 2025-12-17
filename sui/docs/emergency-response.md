## Sui Emergency Response Playbook

> **Owner actions** require OwnerCap ownership. **Operator actions** require OperatorCap ownership. All actions execute immediately with no timelock.

---

## AxelarGateway

### Owner Actions

Actions that can be executed directly from owner wallet (OwnerCap holder).

**Upgrade**

- Use `deploy-contract.js upgrade` command. See [Contract Upgrades](../README.md#contract-upgrades).
- Documentation: [Upgrade Procedures](../README.md#upgrade-procedures)

**Transfer Ownership**

- Transfer OwnerCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

**Transfer UpgradeCap**

- Transfer UpgradeCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

---

## GasService

### Owner Actions

Actions that can be executed directly from owner wallet (OwnerCap holder). Immediate execution, no timelock.

**Upgrade**

- Use `deploy-contract.js upgrade` command. See [Contract Upgrades](../README.md#contract-upgrades).
- Documentation: [Upgrade Procedures](../README.md#upgrade-procedures)

**Transfer Ownership**

- Transfer OwnerCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

**Transfer UpgradeCap**

- Transfer UpgradeCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

### Operator Actions

Actions that can be executed directly from operator wallet (OperatorCap holder). Immediate execution, no timelock.

- **Collect Gas**: `sui/gas-service.js collectGas --amount <amount> --receiver <receiver>` - [Collect Gas](../README.md#collect-gas)
- **Refund Gas**: `sui/gas-service.js refund <messageId> --amount <amount> --receiver <receiver>` - [Refund Gas](../README.md#refund-gas)

---

## InterchainTokenService

### Owner Actions

Actions that can be executed directly from owner wallet (OwnerCap holder). Immediate execution, no timelock.

**Upgrade**

- Use `deploy-contract.js upgrade` command. See [Contract Upgrades](../README.md#contract-upgrades).
- Documentation: [Upgrade Procedures](../README.md#upgrade-procedures)

**Add Trusted Chains**

- `sui/its.js add-trusted-chains <chain1> <chain2> ...` - [Setup Trusted Chains](../README.md#setup-trusted-chains)

**Remove Trusted Chains**

- `sui/its.js remove-trusted-chains <chain1> <chain2> ...` - [Setup Trusted Chains](../README.md#setup-trusted-chains)

**Transfer Ownership**

- Transfer OwnerCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

**Transfer UpgradeCap**

- Transfer UpgradeCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

### Operator Actions

Actions that can be executed directly from operator wallet (OperatorCap holder). Immediate execution, no timelock.

- **Set Flow Limits**: `sui/its.js set-flow-limits <token-ids> <flow-limits>` - [Set Flow Limits](../README.md#set-flow-limits)

---

## Operators Contract

### Owner Actions

Actions that can be executed directly from owner wallet (OwnerCap holder).

- **Add Operator**: `sui/operators.js add <operator>` - [Operator Management](../README.md#operator-management)
- **Remove Operator**: `sui/operators.js remove <operator>` - [Operator Management](../README.md#operator-management)
- **Transfer Ownership**: Transfer OwnerCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).
- **Transfer UpgradeCap**: Transfer UpgradeCap object using `transfer-object.js`. See [Transfer Object](../README.md#transfer-object).

**Upgrade**

- Use `deploy-contract.js upgrade` command. See [Contract Upgrades](../README.md#contract-upgrades).
- Documentation: [Upgrade Procedures](../README.md#upgrade-procedures)

---

## Quick Reference

### Transfer Object (OwnerCap/UpgradeCap)

```bash
ts-node sui/transfer-object.js --contractName <ContractName> --objectName <OwnerCap|UpgradeCap> --recipient <recipient>
```

Or with explicit object ID:

```bash
ts-node sui/transfer-object.js --objectId <objectId> --recipient <recipient>
```

### Upgrade Contract

```bash
ts-node sui/deploy-contract.js upgrade <ContractName> <policy>
```

Where `policy` can be:
- `any_upgrade`: Allow any upgrade
- `code_upgrade`: Upgrade policy to just add code
- `dep_upgrade`: Upgrade policy to just change dependencies

### Post-Upgrade Migration

After upgrading a package, state migrations can be called:

```bash
ts-node sui/deploy-contract.js migrate <ContractName>
```

**Notes**
- All operations execute immediately with no timelock delay.
- OwnerCap and UpgradeCap are Sui objects that can be transferred using `transfer-object.js`.
- Ensure you have the correct capabilities (OwnerCap/OperatorCap) before executing actions.
- For multisig operations, see [Multisig Operations](../README.md#multisig-operations).

---

## Multisig Transaction Execution

For operations requiring multisig approval, follow these steps:

### Step 1: Generate Transaction File

Add `--offline --txFilePath ./tx-upgrade.json --sender $MULTISIG_ADDRESS` to any command to generate an unsigned transaction file:

```bash
ts-node sui/deploy-contract.js upgrade <ContractName> <policy> --offline --txFilePath ./tx-upgrade.json --sender $MULTISIG_ADDRESS
```

Or for other commands:

```bash
ts-node sui/transfer-object.js --objectId <objectId> --recipient <recipient> --offline --txFilePath ./tx-transfer.json --sender $MULTISIG_ADDRESS
```

### Step 2: Sign Transaction

Each signer signs the transaction block using their private key:

```bash
ts-node sui/multisig.js sign --txBlockPath ./tx-upgrade.json --signatureFilePath signature-1.json --offline
```

Repeat for each signer (e.g., `signature-2.json`, `signature-3.json`, etc.).

### Step 3: Combine and Execute

Combine all signatures and execute the transaction:

```bash
ts-node sui/multisig.js combine --txBlockPath ./tx-upgrade.json --signatureFilePath ./combined.json --signatures signature-1.json signature-2.json --executeResultPath ./output.json
```

The `--executeResultPath` option will combine the signatures and execute the transaction in one step. The transaction result will be stored in the specified output file.

**Note:** For more details on multisig setup and operations, see [Multisig Operations](../README.md#multisig-operations).

