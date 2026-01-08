# Upgrade Gateway

## Overview

Upgrade Gateway updates the Gateway contract implementation to a new version. This is used to deploy bug fixes, security patches, and new features.

This is a critical operation that requires careful coordination and testing to ensure system stability.

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

**Required Role:** Gateway Governance (AxelarServiceGovernance)

**Via Governance:**
```bash
ts-node evm/governance.js schedule upgrade <activationTime> \
  --targetContractName AxelarGateway \
  [--implementation <implementationAddress>] \
  --generate-only proposal.json
```

**Note:** Gateway upgrades must go through governance. The `--targetContractName` is required. The `--implementation` flag is optional for `AxelarGateway` (will use implementation from chain config if not provided).

**Examples:**

Schedule upgrade via governance:
```bash
ts-node evm/governance.js schedule upgrade 2025-12-31T12:00:00 \
  --targetContractName AxelarGateway \
  --generate-only proposal.json
```

Schedule upgrade with specific implementation:
```bash
ts-node evm/governance.js schedule upgrade 2025-12-31T12:00:00 \
  --targetContractName AxelarGateway \
  --implementation 0x4F4495243837681061C4743b74B3eEdf548D56A5 \
  --generate-only proposal.json
```

### Stellar

**Required Role:** Gateway Owner

**Upgrade:**
```bash
ts-node stellar/deploy-contract.js upgrade AxelarGateway --version <version> [--migration-data <migrationData>]
```

or

```bash
ts-node stellar/deploy-contract.js upgrade AxelarGateway --artifact-dir <artifactDir> [--migration-data <migrationData>]
```

**Note:** Either `--version` or `--artifact-dir` is required. The `--version` flag specifies a released version (X.Y.Z) or commit hash. The `--artifact-dir` flag points to the contract artifact directory containing the new WASM bytecode. The `--migration-data` flag is optional and defaults to `()` if not provided.

**Examples:**

Upgrade using version:
```bash
ts-node stellar/deploy-contract.js upgrade AxelarGateway --version 2.1.7
```

### Sui

**Required Role:** Gateway Owner (OwnerCap holder)

**Upgrade:**
```bash
ts-node sui/deploy-contract.js upgrade AxelarGateway <policy>
```

**Note:** The `<policy>` argument is required and specifies the upgrade policy. Valid values are: `any_upgrade`, `code_upgrade`, `dep_upgrade`, or `immutable`. The default is `any_upgrade` if not specified during deployment.

**Examples:**

Upgrade with any_upgrade policy:
```bash
ts-node sui/deploy-contract.js upgrade AxelarGateway any_upgrade
```

## Verification

**EVM:** Check implementation address via block explorer or contract query.

**Stellar:** Check contract version via CLI or block explorer.

**Sui:** Check package version via CLI or block explorer.

