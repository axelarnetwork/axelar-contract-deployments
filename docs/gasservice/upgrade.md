# Upgrade GasService

## Overview

Upgrade GasService updates the GasService contract implementation to a new version. This is used to deploy bug fixes, security patches, and new features.

## Routine Operations

- **Scheduled Upgrades**: Planned contract upgrades following release schedules and testing procedures
- **Feature Deployment**: Deploying new features and improvements as part of protocol evolution
- **Maintenance Updates**: Applying bug fixes and optimizations during maintenance windows

## Emergency Scenarios

- **Critical Bug Fix**: Critical bug or vulnerability discovered requiring immediate patch deployment
- **Security Patch**: Security vulnerability requiring urgent contract upgrade
- **Emergency Hotfix**: Urgent fix needed to address active security incidents or operational issues

## Execution

### EVM

**Required Role:** GasService Owner (AxelarServiceGovernance)

**Via Governance:**
```bash
ts-node evm/governance.js schedule upgrade <activationTime> \
  --targetContractName AxelarGasService \
  --implementation <implementationAddress> \
  --generate-only proposal.json
```

**Note:** GasService upgrades must go through governance. The `--targetContractName` is required. The `--implementation` flag is required for `AxelarGasService`.

**Examples:**

Schedule upgrade via governance:
```bash
ts-node evm/governance.js schedule upgrade 2025-12-31T12:00:00 \
  --targetContractName AxelarGasService \
  --implementation 0x2d5d7d31F671F86C782533cc367F14109a082712 \
  --generate-only proposal.json
```

### Stellar

**Required Role:** GasService Owner

**Upgrade:**
```bash
ts-node stellar/deploy-contract.js upgrade AxelarGasService --version <version> [--migration-data <migrationData>]
```

or

```bash
ts-node stellar/deploy-contract.js upgrade AxelarGasService --artifact-dir <artifactDir> [--migration-data <migrationData>]
```

**Note:** Either `--version` or `--artifact-dir` is required. The `--version` flag specifies a released version (X.Y.Z) or commit hash. The `--artifact-dir` flag points to the contract artifact directory containing the new WASM bytecode. The `--migration-data` flag is optional and defaults to `()` if not provided.

**Examples:**

Upgrade GasService:
```bash
ts-node stellar/deploy-contract.js upgrade AxelarGasService --version 2.1.7
```

### Sui

**Required Role:** GasService Owner (OwnerCap holder)

**Upgrade:**
```bash
ts-node sui/deploy-contract.js upgrade GasService <policy>
```

**Note:** The `<policy>` argument is required and specifies the upgrade policy. Valid values are: `any_upgrade`, `code_upgrade`, `dep_upgrade`, or `immutable`. The default is `any_upgrade` if not specified during deployment.

**Examples:**

Upgrade with any_upgrade policy:
```bash
ts-node sui/deploy-contract.js upgrade GasService any_upgrade
```

## Verification

After execution, verify the upgrade:

**EVM:** Check implementation address via block explorer or contract query.

**Stellar:** Check contract version via CLI or block explorer.

**Sui:** Check package version via CLI or block explorer.

