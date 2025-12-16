# Axelar Amplifier Emergency Response Playbook

## 1. Emergency Categories

- **Contract Bug / Exploit Risk** - Critical bug in Router / Gateway / VotingVerifier / MultisigProver / ITS Hub
- **Key / Role Compromise** - Admin EOA, Key Rotation EOA, or Relayer Operators EOA compromised
- **Verifier Set / Signing Incident** - Compromised verifier set, signing issues, threshold adjustments
- **ITS Hub Flow Incident** - Abnormal flows, chain-specific issues (trigger: volume > 3x average of past 5 hours)
- **Configuration Error** - Misconfigured parameters, wrong addresses

---

## 2. Quick Reference

### Emergency Operations

| Operation | Command | Role | Approval |
|-----------|---------|------|----------|
| Freeze chain (Router) | `ts-node cosmwasm/contract.ts router-freeze-chain <chain> -e <env> -y` | Admin EOA | Engineer alone |
| Unfreeze chain (Router) | `ts-node cosmwasm/contract.ts router-unfreeze-chain <chain> -e <env> -y` | Admin EOA | Engineer alone |
| Disable routing (killswitch) | `ts-node cosmwasm/contract.ts router-disable-routing -e <env> -y` | Admin EOA | Leadership |
| Enable routing | `ts-node cosmwasm/contract.ts router-enable-routing -e <env> -y` | Admin EOA | Leadership |
| Disable signing | `ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> -y` | Admin EOA | Engineer alone |
| Enable signing | `ts-node cosmwasm/contract.ts multisig-enable-signing -e <env> -y` | Admin EOA | After verification |
| Freeze chain (ITS) | `ts-node cosmwasm/contract.ts its-freeze-chain <chain> -e <env> -y` | Admin EOA | Engineer alone |
| Unfreeze chain (ITS) | `ts-node cosmwasm/contract.ts its-unfreeze-chain <chain> -e <env> -y` | Admin EOA | Engineer alone |
| Disable ITS execution | `ts-node cosmwasm/contract.ts its-disable-execution -e <env> -y` | Admin EOA | Leadership |
| Enable ITS execution | `ts-node cosmwasm/contract.ts its-enable-execution -e <env> -y` | Admin EOA | Leadership |

**Governance alternative**: Add `--governance -t "Title" -d "Description"` to use governance proposal instead of Admin EOA.

### Verification Queries

| Query | Command |
|-------|---------|
| Router chain frozen | `ts-node cosmwasm/query.ts router-is-chain-frozen <chain> -e <env>` |
| Multisig signing enabled | `ts-node cosmwasm/query.ts multisig-is-signing-enabled -e <env>` |
| ITS chain frozen | `ts-node cosmwasm/query.ts its-is-chain-frozen <chain> -e <env>` |
| ITS execution enabled | `ts-node cosmwasm/query.ts its-is-execution-enabled -e <env>` |
| Contract admin | `ts-node cosmwasm/query.ts contract-admin -c <Contract> -e <env>` |
| Voting threshold | `ts-node cosmwasm/query.ts voting-verifier-threshold <chain> -e <env>` |
| Signing threshold | `ts-node cosmwasm/query.ts multisig-prover-signing-threshold <chain> -e <env>` |
| Current verifier set | `ts-node cosmwasm/query.ts multisig-prover-current-verifier-set <chain> -e <env>` |

---

## 3. Response Playbooks

### 3.1 Contract Bug / Exploit Risk

| Step | Action | SLA | Parallel |
|------|--------|-----|----------|
| 1 | Disable ITS execution | 15 min | No |
| 2 | Freeze chain on Router | 15 min | Yes |
| 3 | Disable signing on Multisig | 15 min | Yes |
| 4 | Deploy and test fix | 24-72h | No |
| 5 | Roll out fix via governance | 24h+ | No |
| 6 | Re-enable operations | 30 min | No |

**Backup**: If freeze fails → disable routing entirely. If governance fails → contact validators to halt chain.

---

### 3.2 Key / Role Compromise

| Step | Action | SLA |
|------|--------|-----|
| 1 | Contain (disable/freeze) | 15 min |
| 2 | Coordinate to ignore malicious proposals | 30 min |
| 3 | Update admin via governance | 24h+ |
| 4 | Restore operations | 30 min |

**Update admin via governance**:
```bash
ts-node cosmwasm/submit-proposal.js executeByGovernance \
  -c <Contract> \
  -t "Update admin" \
  -d "Rotate compromised EOA" \
  --deposit 100000000 \
  --msg '{"update_admin":{"new_admin":"<NEW_ADDRESS>"}}'
```

---

### 3.3 Verifier Set / Signing Incident

| Step | Action | SLA |
|------|--------|-----|
| 1 | Register/Deregister verifiers | 1h |
| 2 | Update verifier set | 1h |
| 3 | Submit proof on destination | 30 min |
| 4 | Confirm rotation | 30 min |

**Update verifier set**:
```bash
ts-node cosmwasm/rotate-signers.js update-verifier-set <chain> -e <env>
ts-node cosmwasm/rotate-signers.js confirm-verifier-rotation <chain> <tx> -e <env>
```

**Update thresholds** (governance required):
```bash
# Voting threshold
ts-node cosmwasm/submit-proposal.js executeByGovernance -c VotingVerifier -n <chain> \
  --msg '{"update_voting_threshold":{"new_voting_threshold":["<num>","<denom>"]}}'

# Signing threshold
ts-node cosmwasm/submit-proposal.js executeByGovernance -c MultisigProver -n <chain> \
  --msg '{"update_signing_threshold":{"new_signing_threshold":["<num>","<denom>"]}}'
```

---

### 3.4 ITS Hub Flow Incident

| Step | Action | SLA |
|------|--------|-----|
| 1 | Freeze affected chain | 15 min |
| 2 | Investigate flow metrics | 2h |
| 3 | Unfreeze after investigation | 30 min |

**Backup**: If freeze fails → disable ITS execution entirely.

---

### 3.5 Configuration Error

| Step | Action | SLA |
|------|--------|-----|
| 1 | Query current config | 30 min |
| 2 | Freeze/pause if unsafe | 15 min |
| 3 | Fix via governance | 24h+ |
| 4 | Verify fix | 30 min |

---

## 4. Escalation Path

| Severity | When | Contact | SLA |
|----------|------|---------|-----|
| **P0** | Active exploit, funds at risk | Security Team Lead + CTO | 15 min |
| **P1** | Key compromise, system-wide disable | Security Team Lead | 30 min |
| **P2** | Single chain freeze, abnormal flow | Engineering Lead | 1 hour |
| **P3** | Config error, non-urgent | Team Lead | 4 hours |

### Decision Authority

| Action | Engineer Alone? | Leadership Required? |
|--------|-----------------|---------------------|
| Freeze single chain | Yes | No |
| Disable signing | Yes | No |
| Disable routing (killswitch) | No | Yes |
| Disable ITS execution | No | Yes |
| Update thresholds | No | Yes |
| Update admin | No | Yes |
| Contract upgrade | No | Yes |

---

## 5. Contract Role Reference

| Contract | Admin | Governance |
|----------|-------|------------|
| **Router** | FreezeChain, UnfreezeChain, DisableRouting, EnableRouting | RegisterChain, UpgradeGateway |
| **Multisig** | DisableSigning, EnableSigning, UnauthorizeCallers | AuthorizeCallers, DisableSigning, EnableSigning |
| **MultisigProver** | UpdateVerifierSet | UpdateSigningThreshold, UpdateAdmin |
| **VotingVerifier** | - | UpdateVotingThreshold |
| **ITS Hub** | FreezeChain, UnfreezeChain, DisableExecution, EnableExecution | RegisterChains, UpdateChains |

---

## 6. Post-Incident Checklist

- [ ] Confirm all pause/freeze states are correct
- [ ] Verify contract states using queries above
- [ ] Document all tx hashes and block heights
- [ ] File incident report with timeline
- [ ] Notify stakeholders within 1 hour
- [ ] Publish post-mortem within 48 hours (if user-facing)
- [ ] Review and update this playbook

---

## 7. Environment Reference

| Environment | Governance Module | Config File |
|-------------|-------------------|-------------|
| mainnet | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar-chains-config/info/mainnet.json` |
| testnet | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar-chains-config/info/testnet.json` |
| stagenet | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar-chains-config/info/stagenet.json` |
| devnet-amplifier | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar-chains-config/info/devnet-amplifier.json` |
