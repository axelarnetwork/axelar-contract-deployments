# Axelar Consensus Chain Emergency Response Playbook

## 1. Emergency Categories

- **Chain Security Incident** - Critical bug in EVM/Axelarnet/Nexus modules, suspicious cross-chain activity
- **Key / Role Compromise** - Governance key (ROLE_ACCESS_CONTROL), Chain management (ROLE_CHAIN_MANAGEMENT)
- **Cross-Chain Transfer Incident** - Abnormal volumes (trigger: > 3x average of past 5 hours)
- **Validator / Signing Incident** - Compromised validator key, key rotation needed
- **Configuration Error** - Misconfigured parameters, wrong gateway addresses

---

## 2. Quick Reference

### Emergency Operations

| Operation | Command | Role | Approval |
|-----------|---------|------|----------|
| Deactivate chain | `axelard tx nexus deactivate-chain <chain>` | ACCESS_CONTROL | Engineer alone |
| Deactivate ALL chains | `axelard tx nexus deactivate-chain :all:` | ACCESS_CONTROL | Leadership |
| Activate chain | `axelard tx nexus activate-chain <chain>` | ACCESS_CONTROL | After verification |
| Disable link-deposit | `axelard tx nexus disable-link-deposit` | ACCESS_CONTROL | Engineer alone |
| Enable link-deposit | `axelard tx nexus enable-link-deposit` | ACCESS_CONTROL | After verification |
| Set rate limit | `axelard tx nexus set-transfer-rate-limit <chain> <amount> <window>` | ACCESS_CONTROL | Leadership |
| Deregister controller | `axelard tx permission deregister-controller <addr>` | ACCESS_CONTROL | Leadership |
| Register controller | `axelard tx permission register-controller <addr>` | ACCESS_CONTROL | Leadership |

**Common flags**: `--from <key> --gas auto --gas-adjustment 1.3 --gas-prices 0.00005uaxl --chain-id axelar-dojo-1 --node <rpc> -y`

### Verification Queries

| Query | Command |
|-------|---------|
| Chain state | `axelard q nexus chain-state <chain>` |
| All chains | `axelard q nexus chains` |
| Link-deposit status | `axelard q nexus link-deposit-enabled` |
| Transfer rate limit | `axelard q nexus transfer-rate-limit <chain> <denom>` |
| Key info | `axelard q multisig key <key-id>` |
| Current key ID | `axelard q multisig key-id <chain>` |
| Gateway address | `axelard q evm gateway-address <chain>` |
| Permission params | `axelard q permission params` |

---

## 3. Response Playbooks

### 3.1 Chain Security Incident

| Step | Action | SLA | Parallel |
|------|--------|-----|----------|
| 1 | Deactivate affected chain(s) | 15 min | No |
| 2 | Disable link-deposit | 15 min | Yes |
| 3 | Set rate limit to 0 | 15 min | Yes |
| 4 | Investigate and develop fix | 24-72h | No |
| 5 | Deploy fix (network upgrade if needed) | 24h+ | No |
| 6 | Re-enable operations | 30 min | No |

**Backup**: If deactivation fails → deactivate ALL chains. If that fails → contact validators to halt.

---

### 3.2 Key / Role Compromise

| Step | Action | SLA |
|------|--------|-----|
| 1 | Contain (deactivate chains) | 15 min |
| 2 | Coordinate to ignore compromised account | 30 min |
| 3 | Deregister compromised controller | 24h+ |
| 4 | Register new controller | 24h+ |
| 5 | Restore operations | 30 min |

**Deregister/Register controller**:
```bash
axelard tx permission deregister-controller <compromised-addr> --from <access-control-key> ...
axelard tx permission register-controller <new-addr> --from <access-control-key> ...
```

---

### 3.3 Key Rotation / Signing Incident

| Step | Action | SLA |
|------|--------|-----|
| 1 | Start keygen | 1h |
| 2 | Validators submit public keys | 1h |
| 3 | Rotate to new key | 1h |
| 4 | Transfer operatorship on destination | 30 min |

**Key rotation**:
```bash
axelard tx multisig start-keygen <new-key-id> --from <chain-management-key> ...
axelard tx multisig rotate-key <chain> <new-key-id> --from <chain-management-key> ...
axelard tx evm transfer-operatorship <chain> <new-key-id> --from <chain-management-key> ...
axelard tx evm sign-commands <chain> --from <any-account> ...
```

---

### 3.4 Cross-Chain Transfer Incident

| Step | Action | SLA |
|------|--------|-----|
| 1 | Deactivate affected chain | 15 min |
| 2 | Set transfer rate limit | 15 min |
| 3 | Investigate flow metrics | 2h |
| 4 | Reactivate after investigation | 30 min |

---

### 3.5 Configuration Error

| Step | Action | SLA |
|------|--------|-----|
| 1 | Query current config | 30 min |
| 2 | Deactivate if unsafe | 15 min |
| 3 | Fix configuration | 24h+ |
| 4 | Verify fix | 30 min |

---

## 4. Escalation Path

| Severity | When | Contact | SLA |
|----------|------|---------|-----|
| **P0** | Active exploit, funds at risk | Security Team Lead + CTO | 15 min |
| **P1** | Key compromise, all-chain deactivation | Security Team Lead | 30 min |
| **P2** | Single chain deactivation, abnormal flow | Engineering Lead | 1 hour |
| **P3** | Config error, non-urgent | Team Lead | 4 hours |

### Decision Authority

| Action | Engineer Alone? | Leadership Required? |
|--------|-----------------|---------------------|
| Deactivate single chain | Yes | No |
| Disable link-deposit | Yes | No |
| Deactivate ALL chains | No | Yes |
| Set transfer rate limit | No | Yes |
| Rotate key | No | Yes |
| Register/Deregister controller | No | Yes |

---

## 5. Permission Roles Reference

| Role | Description | Typical Holder |
|------|-------------|----------------|
| **ROLE_ACCESS_CONTROL** | Highest privilege - critical operations | Governance multisig |
| **ROLE_CHAIN_MANAGEMENT** | Chain ops, key rotation, params | Chain management account |
| **ROLE_UNRESTRICTED** | Normal operations | Any account |

### Module Permissions

| Module | Operations | Required Role |
|--------|------------|---------------|
| **nexus** | ActivateChain, DeactivateChain, SetTransferRateLimit | ACCESS_CONTROL |
| **nexus** | EnableLinkDeposit, DisableLinkDeposit | ACCESS_CONTROL |
| **nexus** | RegisterAssetFee | CHAIN_MANAGEMENT |
| **evm** | SetGateway, AddChain | ACCESS_CONTROL |
| **evm** | CreateDeployToken, CreateTransferOperatorship | CHAIN_MANAGEMENT |
| **multisig** | StartKeygen, RotateKey | CHAIN_MANAGEMENT |
| **permission** | RegisterController, DeregisterController | ACCESS_CONTROL |

---

## 6. Post-Incident Checklist

- [ ] Confirm all chain states are correct
- [ ] Verify activation status using queries above
- [ ] Document all tx hashes and block heights
- [ ] File incident report with timeline
- [ ] Notify stakeholders within 1 hour
- [ ] Publish post-mortem within 48 hours (if user-facing)
- [ ] Review and update this playbook

---

## 7. TODO: Script Support for Consensus Chain Operations

### Current Status

The `cosmwasm/` directory contains scripts for **CosmWasm Amplifier contracts** only (e.g., `contract.ts`, `query.ts`, `rotate-signers.js`). These scripts **cannot** be used for **Axelar Consensus Chain native modules** (`nexus`, `evm`, `multisig`, `permission`).

### Operations Requiring Script Development

#### ROLE_ACCESS_CONTROL Operations (High Priority)

| Operation | Current Command | Priority |
|-----------|-----------------|----------|
| Deactivate chain | `axelard tx nexus deactivate-chain <chain>` | P0 |
| Deactivate all chains | `axelard tx nexus deactivate-chain :all:` | P0 |
| Activate chain | `axelard tx nexus activate-chain <chain>` | P1 |
| Disable link-deposit | `axelard tx nexus disable-link-deposit` | P0 |
| Enable link-deposit | `axelard tx nexus enable-link-deposit` | P1 |
| Set transfer rate limit | `axelard tx nexus set-transfer-rate-limit` | P1 |
| Deregister controller | `axelard tx permission deregister-controller` | P1 |
| Register controller | `axelard tx permission register-controller` | P2 |
| Set gateway address | `axelard tx evm set-gateway` | P2 |

#### ROLE_CHAIN_MANAGEMENT Operations (Medium Priority)

| Operation | Current Command | Priority |
|-----------|-----------------|----------|
| Start keygen | `axelard tx multisig start-keygen` | P2 |
| Rotate key | `axelard tx multisig rotate-key` | P2 |
| Transfer EVM operatorship | `axelard tx evm transfer-operatorship` | P2 |
| Register asset fee | `axelard tx nexus register-asset-fee` | P3 |

#### Query Operations (Medium Priority)

| Query | Current Command | Priority |
|-------|-----------------|----------|
| Chain state | `axelard q nexus chain-state` | P1 |
| All chains | `axelard q nexus chains` | P2 |
| Link-deposit status | `axelard q nexus link-deposit-enabled` | P1 |
| Transfer rate limit | `axelard q nexus transfer-rate-limit` | P2 |
| Key info | `axelard q multisig key` / `key-id` | P2 |
| Gateway address | `axelard q evm gateway-address` | P2 |
| Permission params | `axelard q permission params` | P2 |

**Recommendation**: Create `consensus/` directory with dedicated scripts for these operations.
