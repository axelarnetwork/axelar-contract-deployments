## EVM Emergency Response Playbook

> **Governance actions** go through `evm/governance.js` (Interchain/AxelarServiceGovernance). **Operator actions** use per-contract scripts (e.g., `gateway.js`, `operators.js`, `its.js`).
> Note: Use `evm/governance.js` only for raw proposal creation / proposal submission / timelock flows (schedule/submit/eta/execute) and upgrades. For non-upgrade operational actions, use the contract-specific scripts below.

### Action Execution Method by Contract

#### AxelarGateway

| Action | Execution Method | Script | Notes |
|--------|-----------------|--------|-------|
| Upgrade | Governance | `evm/governance.js` | Requires governance proposal |
| Transfer Ownership | Governance | `evm/governance.js` | Via governance proposal |
| Transfer Operatorship (Owner) | Governance | `evm/governance.js` | Via governance proposal |
| Rotate Signers | Operator | `evm/gateway.js` | Emergency operator action, bypasses delay |
| Transfer Operatorship (Operator) | Operator | `evm/gateway.js` | Direct operator action |

#### AxelarGasService

| Action | Execution Method | Script | Notes |
|--------|-----------------|--------|-------|
| Upgrade | Governance | `evm/governance.js` | Requires governance proposal |

#### InterchainTokenService

| Action | Execution Method | Script | Notes |
|--------|-----------------|--------|-------|
| Upgrade | Governance | `evm/governance.js` | Requires governance proposal |
| Set/Remove Trusted Chains | Governance | `evm/its.js` | Via `evm/its.js` with `--governance` flag |
| Set Pause Status | Governance | `evm/its.js` | Via `evm/its.js` with `--governance` flag |
| Migrate Interchain Token | Governance | `evm/its.js` | Via `evm/its.js` with `--governance` flag |
| Set Flow Limits / Freeze Tokens | Operator | `evm/its.js` | Rate limiter operator action |
| Transfer Operatorship (Operator) | Operator | `evm/its.js` | Direct operator action |
| Propose Operatorship (Operator) | Operator | `evm/its.js` | Direct operator action |

#### Operators Contract

| Action | Execution Method | Script | Notes |
|--------|-----------------|--------|-------|
| Add/Remove Operators | Owner | `evm/operators.js` | Owner action |
| Transfer Ownership | Owner | `evm/operators.js` | Owner action |
| Propose Ownership | Owner | `evm/operators.js` | Owner action |
| Execute Contract | Operator | `evm/operators.js` | Operators role action |

**Key Differences:**
- **Governance**: Requires proposal submission, voting period, timelock delay. Use `evm/governance.js schedule` or contract scripts with `--governance` flag.
- **Owner/Operator**: Direct execution from owner/operator wallet. Immediate execution, no timelock. Use contract-specific scripts directly.

---

## 1. Contract Vulnerability: Upgrade or Pause

When a contract vulnerability is discovered, immediately upgrade to a patched version or pause the contract to prevent exploitation.

### Upgrade Contracts

- **AxelarGateway Upgrade**: See [Gateway Upgrade](./README.md#gateway-upgrade) section and [Governance](./README.md#governance) flow
- **AxelarGasService Upgrade**: See [AxelarGasService and AxelarDepositService](./README.md#axelargasservice-and-axelardepositservice) section and [Governance](./README.md#governance) flow
- **InterchainTokenService Upgrade**: See [Governance](./README.md#governance) flow for upgrade via `evm/governance.js`

### Pause Contracts

- **InterchainTokenService Pause**: See [Set pause status (pause/unpause)](./docs/pause-unpause.md)
  - Use `--pauseStatus true` to pause, `false` to unpause after fix

---

## 2. Token Hack or Exploit: Freeze Tokens / Set Flow Limits

When a token is compromised or being exploited, immediately freeze transfers by setting flow limits to prevent further damage.

### Freeze Token Transfers

- **Freeze Tokens (Flow Limit = 1)**: See [Freeze tokens (flow limit = 1)](./docs/rate-limits.md#4.freeze-tokens-on-a-chain)
  - Sets flow limit to 1, effectively freezing transfers
- **Set Flow Limit (Custom)**: See [Set flow limit (single)](./docs/rate-limits.md#3.set-low-limit-(single-token))
  - Set specific flow limit value for a token
- **Unfreeze Tokens (Flow Limit = 0)**: See [Unfreeze tokens (flow limit = 0)](./docs/rate-limits.md#4.unfreeze-tokens-on-a-chain)
  - Restore normal operations after incident resolution

### Isolate Affected Chains

- **Remove Trusted Chain**: See [Remove trusted chains](./README.md#remove-trusted-chains)
  - Remove compromised or affected chain from trusted list

---

## 3. Key Compromise: Transfer Roles and Ownership

When a key is compromised, immediately transfer ownership/operatorship to secure addresses.

### Transfer Ownership

- **AxelarGateway Transfer Ownership**: See [Gateway operator commands](./README.md#gateway-operator-commands-evmgatewayjs) (use `evm/gateway.js` or governance via `evm/ownership.js`)
- **AxelarGateway Propose Ownership**: See [Gateway operator commands](./README.md#gateway-operator-commands-evmgatewayjs) (use `evm/gateway.js` or governance via `evm/ownership.js`)
- **Operators Contract Transfer Ownership**: See [Transfer ownership](./README.md#transfer-ownership)
- **Operators Contract Propose Ownership**: See [Propose ownership](./README.md#propose-ownership)

### Transfer Operatorship

- **AxelarGateway Transfer Operatorship**: See [Transfer operatorship](./README.md#transfer-operatorship) in Gateway operator commands
- **InterchainTokenService Transfer Operatorship**: See [Transfer operatorship](./README.md#transfer-operatorship-1) in ITS operator commands

### Rotate Signers (Gateway Emergency)

- **Rotate Signers**: See [Rotate signers (emergency)](./README.md#rotate-signers-emergency)
  - Can bypass minimum rotation delay for emergency recovery

### Manage Operators

- **Add Operator**: See [Add operator](./README.md#add-operator)
- **Remove Operator**: See [Remove operator](./README.md#remove-operator)
  - Remove compromised operator immediately

---

## Governance Execution Flow (for upgrade/pause actions)

When using governance for upgrades or pause actions:

1. **Schedule**: `evm/governance.js schedule ... <ETA> ... [--file proposal.json]`
2. **Submit**: auto if `MNEMONIC` set; or submit `proposal.json` via Cosmos CLI
3. **Wait** for vote + GMP; if relayers fail, use `evm/governance.js submit ...`
4. **ETA**: `evm/governance.js eta --target <addr> --calldata <0x...> [--nativeValue <wei>]`
5. **Execute**: `evm/governance.js execute --target <addr> --calldata <0x...> [--nativeValue <wei>]`

**Notes**
- `--nativeValue` must match the value used when scheduling (hash includes it).
- Use `-c AxelarServiceGovernance` for ITS/Amp/Gas actions.
- Use `--file proposal.json` to inspect calldata/payload before submit.
