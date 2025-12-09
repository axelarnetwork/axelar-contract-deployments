## EVM Emergency Response Playbook

> **Governance actions** go through `evm/governance.js` (Interchain/AxelarServiceGovernance). **Owner/Operator actions** use per-contract scripts (e.g., `gateway.js`, `operators.js`, `its.js`).
> Note: Use `evm/governance.js` only for raw proposal creation / proposal submission / timelock flows (schedule/submit/eta/execute) and upgrades. For non-upgrade operational actions, use the contract-specific scripts below.

---

## Governance Actions

Actions that require governance proposal submission, voting period, and timelock delay. Use `evm/governance.js schedule` or contract scripts with `--governance` flag.

### AxelarGateway

| Action | Script | Documentation |
|--------|--------|---------------|
| Upgrade | `evm/governance.js` | [Upgrade Workflow](./docs/governance-workflows.md#upgrade-workflow) |
| Transfer Ownership | `evm/governance.js` | [Transfer Gateway Governance via Governance](./docs/governance-workflows.md#transfer-gateway-governance-via-governance) |
| Transfer Operatorship (Owner) | `evm/governance.js` | [Transfer Gateway Operatorship via Governance](./docs/governance-workflows.md#transfer-gateway-operatorship-via-governance-amplifier) |

### AxelarGasService

| Action | Script | Documentation |
|--------|--------|---------------|
| Upgrade | `evm/governance.js` | [Upgrade Workflow](./docs/governance-workflows.md#upgrade-workflow) |

### InterchainTokenService

| Action | Script | Documentation |
|--------|--------|---------------|
| Upgrade | `evm/governance.js` | [Upgrade Workflow](./docs/governance-workflows.md#upgrade-workflow) |
| Set/Remove Trusted Chains | `evm/its.js` (with `--governance` flag) | [Set Trusted Chains via ITS](./docs/governance-workflows.md#set-trusted-chains-via-its) / [Remove Trusted Chains via ITS](./docs/governance-workflows.md#remove-trusted-chains-via-its) |
| Set Pause Status | `evm/its.js` (with `--governance` flag) | [Pause / Unpause ITS](./docs/governance-workflows.md#pause--unpause-its) |
| Migrate Interchain Token | `evm/its.js` (with `--governance` flag) | [Migrate Interchain Token via ITS](./docs/governance-workflows.md#migrate-interchain-token-via-its) |

---

## Owner/Operator Actions

Actions that can be executed directly from owner/operator wallet. Immediate execution, no timelock. Use contract-specific scripts directly.

### AxelarGateway

| Action | Role | Script | Documentation |
|--------|------|--------|---------------|
| Rotate Signers | Operator | `evm/gateway.js` | [Rotate signers (emergency)](./README.md#rotate-signers-emergency) |
| Transfer Operatorship | Operator | `evm/gateway.js` | [Transfer operatorship](./README.md#transfer-operatorship) in Gateway operator commands |

### InterchainTokenService

| Action | Role | Script | Documentation |
|--------|------|--------|---------------|
| Set Flow Limits / Freeze Tokens | Operator | `evm/its.js` | [Set flow limit (single)](./docs/rate-limits.md#3.set-low-limit-(single-token)) / [Freeze tokens](./docs/rate-limits.md#4.freeze-tokens-on-a-chain) / [Unfreeze tokens](./docs/rate-limits.md#4.unfreeze-tokens-on-a-chain) |
| Transfer Operatorship | Operator | `evm/its.js` | [Transfer operatorship](./README.md#transfer-operatorship-1) in ITS operator commands |
| Propose Operatorship | Operator | `evm/its.js` | [Propose operatorship](./README.md#propose-operatorship) |

### Operators Contract

| Action | Role | Script | Documentation |
|--------|------|--------|---------------|
| Add/Remove Operators | Owner | `evm/operators.js` | [Add operator](./README.md#add-operator) / [Remove operator](./README.md#remove-operator) |
| Transfer Ownership | Owner | `evm/operators.js` | [Transfer ownership](./README.md#transfer-ownership) |
| Propose Ownership | Owner | `evm/operators.js` | [Propose ownership](./README.md#propose-ownership) |
| Execute Contract | Operator | `evm/operators.js` | [Execute contract (operators role)](./README.md#execute-contract-operators-role) |

---

## Governance Execution Flow

For complete governance workflow details, see [Governance Workflows](./docs/governance-workflows.md) and [Governance Documentation](./docs/governance.md).

**Quick Reference:**
1. **Schedule**: `evm/governance.js schedule ... <ETA> ... [--file proposal.json]`
2. **Submit**: auto if `MNEMONIC` set; or submit `proposal.json` via Cosmos CLI (see [Submission Methods](./docs/governance.md#submission-methods))
3. **Wait** for vote + GMP; if relayers fail, use `evm/governance.js submit ...` (see [Steps After Scheduling](./docs/governance-workflows.md#steps-after-scheduling-a-proposal))
4. **ETA**: `evm/governance.js eta --target <addr> --calldata <0x...> [--nativeValue <wei>]` (see [Check Proposal ETA](./docs/governance.md#check-proposal-eta))
5. **Execute**: `evm/governance.js execute --target <addr> --calldata <0x...> [--nativeValue <wei>]` (see [Execute Proposal](./docs/governance.md#execute-proposal))

**Notes**
- `--nativeValue` must match the value used when scheduling (hash includes it).
- Use `-c AxelarServiceGovernance` for ITS/Amp/Gas actions.
- Use `--file proposal.json` to inspect calldata/payload before submit.
