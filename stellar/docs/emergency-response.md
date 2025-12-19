# Stellar Emergency Response Playbook

## 1. Emergency Categories

- **Contract Bug / Exploit Risk** - Critical bug in Gateway / ITS / GasService contracts
- **Key / Role Compromise** - Owner EOA, Operator EOA, or Emergency Operator compromised
- **Signer Set / Signing Incident** - Compromised signers, rotation needed
- **ITS Flow Incident** - Abnormal token flows, chain-specific issues (trigger: volume > 3x average of past 5 hours)
- **Configuration Error** - Misconfigured parameters, wrong trusted chains

---

## 2. Quick Reference

### Emergency Operations

| Operation | Command | Role | Approval |
|-----------|---------|------|----------|
| Pause Gateway | `ts-node stellar/contract.js pause AxelarGateway -e <env> --chain-name <chain> -y` | Owner | Engineer alone |
| Unpause Gateway | `ts-node stellar/contract.js unpause AxelarGateway -e <env> --chain-name <chain> -y` | Owner | After verification |
| Pause ITS | `ts-node stellar/contract.js pause InterchainTokenService -e <env> --chain-name <chain> -y` | Owner | Engineer alone |
| Unpause ITS | `ts-node stellar/contract.js unpause InterchainTokenService -e <env> --chain-name <chain> -y` | Owner | After verification |
| Rotate signers | `ts-node stellar/gateway.js rotate -e <env> --chain-name <chain> --signers <signers-json> -y` | Operator | Leadership |
| Set flow limit | `ts-node stellar/its.js set-flow-limit <tokenId> <limit> -e <env> --chain-name <chain> -y` | Operator | Engineer alone |
| Remove flow limit | `ts-node stellar/its.js remove-flow-limit <tokenId> -e <env> --chain-name <chain> -y` | Operator | After verification |
| Remove trusted chain | `ts-node stellar/its.js remove-trusted-chains <trustedChain> -e <env> --chain-name <chain> -y` | Operator | Leadership |
| Add trusted chain | `ts-node stellar/its.js add-trusted-chains <trustedChain> -e <env> --chain-name <chain> -y` | Operator | After verification |

### Verification Queries

| Query | Command |
|-------|---------|
| Gateway paused | `ts-node stellar/contract.js paused AxelarGateway -e <env> --chain-name <chain>` |
| ITS paused | `ts-node stellar/contract.js paused InterchainTokenService -e <env> --chain-name <chain>` |
| Gateway owner | `ts-node stellar/contract.js owner AxelarGateway -e <env> --chain-name <chain>` |
| Gateway operator | `ts-node stellar/contract.js operator AxelarGateway -e <env> --chain-name <chain>` |
| ITS owner | `ts-node stellar/contract.js owner InterchainTokenService -e <env> --chain-name <chain>` |
| ITS operator | `ts-node stellar/contract.js operator InterchainTokenService -e <env> --chain-name <chain>` |
| Is trusted chain | `ts-node stellar/its.js is-trusted-chain <trustedChain> -e <env> --chain-name <chain>` |
| Flow limit | `ts-node stellar/its.js flow-limit <tokenId> -e <env> --chain-name <chain>` |
| Is operator | `ts-node stellar/operators.js is-operator <address> -e <env> --chain-name <chain>` |

---

## 3. Response Playbooks

### 3.1 Contract Bug / Exploit Risk

| Step | Action | SLA | Parallel |
|------|--------|-----|----------|
| 1 | Pause Gateway | 15 min | No |
| 2 | Pause ITS | 15 min | Yes |
| 3 | Set flow limits to 0 for affected tokens | 15 min | Yes |
| 4 | Investigate and develop fix | 24-72h | No |
| 5 | Deploy fix (contract upgrade) | 24h+ | No |
| 6 | Unpause contracts | 30 min | No |

**Backup**: If pause fails → remove all trusted chains on ITS. If that fails → coordinate with validators.

---

### 3.2 Key / Role Compromise

| Step | Action | SLA |
|------|--------|-----|
| 1 | Contain (pause contracts) | 15 min |
| 2 | Coordinate to ignore compromised account | 30 min |
| 3 | Transfer ownership/operatorship | 1h |
| 4 | Restore operations | 30 min |

**Transfer ownership**:
```bash
# Transfer Gateway ownership
ts-node stellar/contract.js transfer-ownership AxelarGateway <NEW_OWNER> -e <env> --chain-name <chain> -y

# Transfer ITS ownership  
ts-node stellar/contract.js transfer-ownership InterchainTokenService <NEW_OWNER> -e <env> --chain-name <chain> -y

# Transfer operatorship
ts-node stellar/contract.js transfer-operatorship AxelarGateway <NEW_OPERATOR> -e <env> --chain-name <chain> -y
ts-node stellar/contract.js transfer-operatorship InterchainTokenService <NEW_OPERATOR> -e <env> --chain-name <chain> -y
```

---

### 3.3 Signer Set / Signing Incident

| Step | Action | SLA |
|------|--------|-----|
| 1 | Pause Gateway if active exploit | 15 min |
| 2 | Prepare new signer set | 1h |
| 3 | Rotate signers | 30 min |
| 4 | Verify rotation | 30 min |

**Rotate signers**:
```bash
# Using Amplifier proof
ts-node stellar/gateway.js submit-proof <multisigSessionId> -e <env> --chain-name <chain> -y

# Manual rotation (requires operator key)
ts-node stellar/gateway.js rotate -e <env> --chain-name <chain> -y \
  --signers '<signers-json>' \
  --current-nonce <current-nonce> \
  --new-nonce <new-nonce>
```

---

### 3.4 ITS Flow Incident

| Step | Action | SLA |
|------|--------|-----|
| 1 | Set flow limit to 0 for affected token | 15 min |
| 2 | Investigate flow metrics | 2h |
| 3 | Restore flow limit after investigation | 30 min |

**Backup**: If flow limit fails → pause ITS entirely.

---

### 3.5 Configuration Error

| Step | Action | SLA |
|------|--------|-----|
| 1 | Query current config | 30 min |
| 2 | Pause if unsafe | 15 min |
| 3 | Fix configuration | 1h |
| 4 | Verify fix | 30 min |

---

## 4. Escalation Path

| Severity | When | Contact | SLA |
|----------|------|---------|-----|
| **P0** | Active exploit, funds at risk | Security Team Lead + CTO | 15 min |
| **P1** | Key compromise, contract paused | Security Team Lead | 30 min |
| **P2** | Flow limit triggered, single token | Engineering Lead | 1 hour |
| **P3** | Config error, non-urgent | Team Lead | 4 hours |

### Decision Authority

| Action | Engineer Alone? | Leadership Required? |
|--------|-----------------|---------------------|
| Pause Gateway/ITS | Yes | No |
| Set flow limit to 0 | Yes | No |
| Unpause contracts | No | Yes |
| Rotate signers | No | Yes |
| Remove trusted chain | No | Yes |
| Transfer ownership | No | Yes |
| Contract upgrade | No | Yes |

---

## 5. Contract Role Reference

| Contract | Owner Operations | Operator Operations |
|----------|------------------|---------------------|
| **AxelarGateway** | `pause`, `unpause`, `transfer_ownership`, `upgrade` | `rotate_signers`, `transfer_operatorship` |
| **AxelarOperators** | `add_operator`, `remove_operator`, `transfer_ownership`, `upgrade` | (no operator role) |
| **AxelarGasService** | `transfer_ownership`, `upgrade` | `collect_fees`, `refund`, `transfer_operatorship` |
| **InterchainTokenService** | `pause`, `unpause`, `transfer_token_admin`, `transfer_ownership`, `upgrade` | `set_trusted_chain`, `remove_trusted_chain`, `set_flow_limit`, `transfer_operatorship` |

**Note**: Unlike EVM contracts, Stellar `pause/unpause` functions are owner-only operations. This will be migrated to operator-accessible in a future upgrade.

See [Stellar Role Transfers Release Doc](../../releases/stellar/2025-11-Stellar-Role-Transfers-Release-v1.0.0.md) for complete role assignments.

---

## 6. Post-Incident Checklist

- [ ] Confirm all pause states are correct
- [ ] Verify contract states using queries above
- [ ] Document all tx hashes and ledger numbers
- [ ] File incident report with timeline
- [ ] Notify stakeholders within 1 hour
- [ ] Publish post-mortem within 48 hours (if user-facing)
- [ ] Review and update this playbook

---

## 7. Environment Reference

| Environment | Chain Name | Config File |
|-------------|------------|-------------|
| mainnet | `stellar` | `axelar-chains-config/info/mainnet.json` |
| testnet | `stellar-2025-q3` | `axelar-chains-config/info/testnet.json` |
| stagenet | `stellar-2025-q3` | `axelar-chains-config/info/stagenet.json` |

**Note**: Stellar testnet resets every quarter. Chain names may change accordingly.
