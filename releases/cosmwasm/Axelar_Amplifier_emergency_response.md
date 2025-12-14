# Axelar Amplifier Emergency Response Playbook

## 1. Emergency Categories

- **Contract Bug / Exploit Risk**
  - Critical bug found in Router / Gateway / VotingVerifier / MultisigProver / ITS Hub.
  - Suspicious behavior suggesting an in-progress exploit.

- **Key / Role Compromise**
  - Compromise or suspected compromise of:
    - Admin EOA (Emergency Operator) - for Router, Multisig, ITS Hub
    - Key Rotation EOA (MultisigProver admin) - for verifier set updates
    - Relayer Operators EOA - for ITS token operations

- **Verifier Set / Signing Related Incident**
  - Compromised verifier set needing rotation.
  - Need to disable signing on Multisig.
  - Voting/signing threshold adjustments needed.

- **ITS Hub Flow-Related Incident**
  - Abnormal message flows on specific chains.
  - Need to freeze chains or disable execution without fully stopping the system.
  - **Trigger threshold**: Volume spike > 3x average of past 5 hours

- **Configuration / Deployment Error**
  - Misconfigured chain parameters, wrong addresses, or bad deployment.
  - Incorrect governance / admin parameters.

---

## 2. Escalation Path & Decision Authority

### Who Can Approve Emergency Actions?

| Action Category | Engineer Can Do Alone? | Requires Leadership Approval? | Notes |
|-----------------|------------------------|------------------------------|-------|
| **Freeze single chain** (Router/ITS) | ✅ Yes | No | Rapid response needed, can be reversed |
| **Unfreeze chain** | ✅ Yes | No | Restoring normal operations |
| **Disable signing** (Multisig) | ✅ Yes | No | Emergency containment |
| **Enable signing** | ⚠️ After verification | Recommended | Confirm threat is resolved first |
| **Disable routing** (killswitch) | ❌ No | ✅ Required | Affects ALL chains, major impact |
| **Enable routing** | ⚠️ After verification | ✅ Required | Restoring full system |
| **Disable ITS execution** | ❌ No | ✅ Required | Affects ALL ITS operations |
| **Enable ITS execution** | ⚠️ After verification | ✅ Required | Restoring full system |
| **Update verifier set** | ❌ No | ✅ Required | Security-critical change |
| **Update thresholds** (voting/signing) | ❌ No | ✅ Required | Security-critical change |
| **Update admin address** | ❌ No | ✅ Required | Security-critical change |
| **Contract migration/upgrade** | ❌ No | ✅ Required | Requires governance proposal |

### Escalation Contacts

| Severity | When to Escalate | Contact | Response Time SLA |
|----------|------------------|---------|-------------------|
| **P0 - Critical** | Active exploit, funds at risk | Security Team Lead + CTO | 15 min |
| **P1 - High** | Key compromise, system-wide disable | Security Team Lead | 30 min |
| **P2 - Medium** | Single chain freeze, abnormal flow | Engineering Lead | 1 hour |
| **P3 - Low** | Config error, non-urgent fix | Team Lead | 4 hours |

### Decision Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     INCIDENT DETECTED                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Is it a single chain issue that can be reversed?               │
│  (freeze chain, disable signing for one chain)                  │
└─────────────────────────────────────────────────────────────────┘
          │                                    │
         YES                                  NO
          │                                    │
          ▼                                    ▼
┌─────────────────────┐          ┌─────────────────────────────────┐
│ Engineer executes   │          │ Escalate to Leadership          │
│ immediately         │          │ (disable routing, disable ITS,  │
│ (notify team after) │          │  threshold changes, etc.)       │
└─────────────────────┘          └─────────────────────────────────┘
                                               │
                                               ▼
                                 ┌─────────────────────────────────┐
                                 │ Leadership approves?            │
                                 └─────────────────────────────────┘
                                       │              │
                                      YES            NO
                                       │              │
                                       ▼              ▼
                                 ┌──────────┐  ┌──────────────────┐
                                 │ Execute  │  │ Explore alternate │
                                 │ action   │  │ solutions         │
                                 └──────────┘  └──────────────────┘
```

### Contract Role Reference (for execution)

| Contract | Admin | Governance |
|----------|-------|------------|
| **Router** | FreezeChain, UnfreezeChain, DisableRouting, EnableRouting | RegisterChain, UpgradeGateway |
| **Multisig** | UnauthorizeCallers, DisableSigning, EnableSigning | AuthorizeCallers, UnauthorizeCallers, DisableSigning, EnableSigning |
| **MultisigProver** | UpdateVerifierSet | UpdateSigningThreshold, UpdateAdmin, UpdateVerifierSet |
| **VotingVerifier** | - | UpdateVotingThreshold |
| **InterchainTokenService** | FreezeChain, UnfreezeChain, DisableExecution, EnableExecution, RegisterP2pTokenInstance, ModifySupply | RegisterChains, UpdateChains, FreezeChain, UnfreezeChain, DisableExecution, EnableExecution |
| **XrplVotingVerifier** | EnableExecution, DisableExecution, UpdateAdmin | UpdateVotingThreshold, EnableExecution, DisableExecution, UpdateAdmin |
| **XrplGateway** | RegisterTokenMetadata, RegisterLocalToken, RegisterRemoteToken, LinkToken, DeployRemoteToken, EnableExecution, DisableExecution, UpdateAdmin | RegisterTokenMetadata, RegisterLocalToken, RegisterRemoteToken, LinkToken, DeployRemoteToken, EnableExecution, DisableExecution, UpdateAdmin |
| **XrplMultisigProver** | UpdateVerifierSet, TrustSet, UpdateFeeReserve, UpdateXrplTransactionFee, UpdateXrplReserves, EnableExecution, DisableExecution, UpdateAdmin | UpdateSigningThreshold, UpdateVerifierSet, TrustSet, UpdateFeeReserve, UpdateXrplTransactionFee, UpdateXrplReserves, EnableExecution, DisableExecution, UpdateAdmin |

**Note**: If an operation appears in both columns, either Admin EOA or Governance proposal can be used.

---

## 3. Response Playbooks by Incident Type

### 3.1 Contract Bug / Exploit Risk

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Disable execution on ITS Hub | 15 min | No | Emergency Operator / Governance |
| 2 | Freeze specific chain on Router | 15 min | Yes (with step 1) | Emergency Operator / Governance |
| 3 | Disable signing on Multisig | 15 min | Yes (with step 1,2) | Emergency Operator / Governance |
| 4 | Deploy and test fix | 24-72h | No | Engineering |
| 5 | Roll out fix via governance | 24h+ | No | Governance |
| 6 | Re-enable operations | 30 min | No | Emergency Operator / Governance |

#### Step 1: Disable execution on ITS Hub (if ITS-related)

**✅ Both Admin EOA and Governance can do this** - Use Admin for faster response

**Option A: Direct execution with Admin EOA (Recommended for emergency):**
```bash
ts-node cosmwasm/contract.ts its-disable-execution -e <env> -y
```

**Option B: Via Governance proposal (if Admin key unavailable):**
```bash
ts-node cosmwasm/contract.ts its-disable-execution -e <env> --governance \
  -t "Emergency: Disable ITS execution" \
  -d "Disable execution due to suspected exploit"
```

**Verification:**
```bash
ts-node cosmwasm/query.ts its-is-execution-enabled -e <env>
# Expected output: false
```

**Backup Action:** If governance proposal fails, contact validators to halt the chain.

---

#### Step 2: Freeze specific chain on Router (if chain-specific) [PARALLEL with Step 1]

**⚠️ Requires Admin EOA** - Cannot use governance proposal

**Action (Direct execution with admin EOA):**
```bash
ts-node cosmwasm/contract.ts router-freeze-chain <chain-name> -e <env> -y
```

**Verification:**
```bash
ts-node cosmwasm/query.ts router-is-chain-frozen <chain-name> -e <env>
# Expected output: true
```

**Backup Action:** If freeze fails, disable routing entirely via `router-disable-routing`.

---

#### Step 3: Disable signing on Multisig [PARALLEL with Step 1,2]

**✅ Both Admin EOA and Governance can do this** - Use Admin for faster response

**Option A: Direct execution with Admin EOA (Recommended for emergency):**
```bash
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> --governance \
  -t "Emergency: Disable signing" \
  -d "Disable signing due to suspected exploit"
```

**Verification:**
```bash
ts-node cosmwasm/query.ts multisig-is-signing-enabled -e <env>
# Expected output: false
```

**Backup Action:** If disable fails, contact validators to halt the chain.

---

#### Step 4: Deploy and test a fix

**Action:** Validate the mitigation on devnet-amplifier / testnet / stagenet before mainnet.

---

#### Step 5: Roll out the fix using governance

**Action:**
```bash
# Upload new contract code
ts-node cosmwasm/submit-proposal.js store -c <ContractName> \
  -t "Upgrade <ContractName>" \
  -d "Security fix for <ContractName>" \
  --deposit 100000000

# After code is stored, migrate the contract
ts-node cosmwasm/submit-proposal.js migrate \
  -c <ContractName> \
  -n <chain-name> \
  --msg '{}' \
  --fetchCodeId
```

**Verification:**
```bash
ts-node cosmwasm/query.ts contract-info -c <ContractName>
# Verify version matches expected
```

---

#### Step 6: Re-enable execution once safe

**✅ Both Admin EOA and Governance can do this**

**Option A: Direct execution with Admin EOA:**
```bash
ts-node cosmwasm/contract.ts its-enable-execution -e <env> -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/contract.ts its-enable-execution -e <env> --governance \
  -t "Re-enable ITS execution" \
  -d "Re-enable execution after fix deployed"
```

**Verification:**
```bash
ts-node cosmwasm/query.ts its-is-execution-enabled -e <env>
# Expected output: true
```

---

### 3.2 Key / Role Compromise

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Contain blast radius (disable/freeze) | 15 min | No | Emergency Operator |
| 2 | Coordinate to ignore malicious proposals | 30 min | Yes (with step 1) | Security Team |
| 3 | Update admin/operator address | 24h+ | No | Governance |
| 4 | Restore operations | 30 min | No | Emergency Operator |

#### Governance Multisig Signer Compromised

**Step 1: Coordinate** (SLA: 30 min)
1. Notify all other signers to ignore proposals not officially announced.
2. If a malicious proposal already has signatures, coordinate to not reach threshold.

**Step 2: Generate new multisig** (SLA: 24h)
- Generate a new governance multisig with new signers.
- Submit proposals to update `governanceAddress` on affected contracts.

**Verification:** Query each contract's governance address to confirm update.

---

#### Emergency Operator (Admin) EOA Compromised

**Step 1: Contain** (SLA: 15 min)

**✅ Both Admin EOA and Governance can do `disable_signing`**

If the compromised admin can still cause damage, use governance to disable signing:

```bash
# Option A: Via Governance proposal (if admin is compromised)
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> --governance \
  -t "Emergency: Disable signing" \
  -d "Contain compromised admin"

# Option B: If using a different trusted admin EOA
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> -y
```

**Verification:**
```bash
ts-node cosmwasm/query.ts multisig-is-signing-enabled -e <env>
```

**Step 2: Update admin address via governance** (SLA: 24h+)
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c <ContractName> \
  -t "Update admin address for <ContractName>" \
  -d "Rotate compromised admin EOA" \
  --deposit 100000000 \
  --msg '{"update_admin":{"new_admin":"<NEW_ADMIN_ADDRESS>"}}'
```

**Verification:**
```bash
ts-node cosmwasm/query.ts contract-admin -c <ContractName> -e <env>
# Confirm new admin address
```

**Backup Action:** If governance is too slow, contact validators to halt the chain.

---

#### Key Rotation EOA (MultisigProver Admin) Compromised

**Step 1: Disable signing** (SLA: 15 min) [PARALLEL with Step 2]

**✅ Both Admin EOA and Governance can do this** - Use Admin for faster response
```bash
# Option A: Direct execution with Admin EOA (faster)
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> -y

# Option B: Via Governance proposal
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> --governance \
  -t "Emergency: Disable signing" \
  -d "Disable signing due to compromised key rotation EOA"
```

**Step 2: Update MultisigProver admin via Governance** (SLA: 24h+)

✅ This CAN use governance proposal (governance has permission to update admin)
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c MultisigProver \
  -n <chain-name> \
  -t "Update MultisigProver admin for <chain>" \
  -d "Rotate compromised key rotation EOA" \
  --deposit 100000000 \
  --msg '{"update_admin":{"new_admin":"<NEW_ADMIN_ADDRESS>"}}'
```

**Verification:**
```bash
ts-node cosmwasm/query.ts contract-admin -c MultisigProver -n <chain-name> -e <env>
```

**Step 3: Re-enable signing** (SLA: 30 min after Step 2)

**✅ Both Admin EOA and Governance can do this**
```bash
# Option A: Direct execution with Admin EOA
ts-node cosmwasm/contract.ts multisig-enable-signing -e <env> -y

# Option B: Via Governance proposal
ts-node cosmwasm/contract.ts multisig-enable-signing -e <env> --governance \
  -t "Re-enable signing" \
  -d "Re-enable signing after admin rotation"
```

---

### 3.3 Verifier Set / Signing Related Incident

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Register/Deregister verifiers | 1h | No | Operations |
| 2 | Update verifier set | 1h | No | Key Rotation EOA |
| 3 | Submit proof on destination | 30 min | No | Operations |
| 4 | Confirm rotation | 30 min | No | Operations |

#### Rotate Verifier Set

**Step 1: Register/Deregister verifiers** (SLA: 1h)
```bash
ampd register-chain-support <service-name> <chain-name>
# or
ampd deregister-chain-support <service-name> <chain-name>
```

**Step 2: Update verifier set** (SLA: 1h)
```bash
ts-node cosmwasm/rotate-signers.js update-verifier-set <chain-name> -e <env>
```

**Verification:**
```bash
ts-node cosmwasm/query.ts multisig-prover-next-verifier-set <chain-name> -e <env>
```

**Step 3: Submit proof on destination chain** (SLA: 30 min)
- For Sui: `ts-node sui/gateway.js submitProof <multisig-session-id>`
- For EVM: `ts-node evm/gateway.js --action submitProof --multisigSessionId <multisig-session-id> -n <chain-name>`

**Step 4: Confirm verifier rotation** (SLA: 30 min)
```bash
ts-node cosmwasm/rotate-signers.js confirm-verifier-rotation <chain-name> <rotate-signers-tx> -e <env>
```

**Verification:**
```bash
ts-node cosmwasm/query.ts multisig-prover-current-verifier-set <chain-name> -e <env>
```

---

#### Update Voting Threshold (VotingVerifier)

**Action:** (SLA: 24h+ for governance)
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c VotingVerifier \
  -n <chain-name> \
  -t "Update voting threshold for <chain>" \
  -d "Update voting threshold" \
  --deposit 100000000 \
  --msg '{"update_voting_threshold":{"new_voting_threshold":["<numerator>","<denominator>"]}}'
```

**Verification:**
```bash
ts-node cosmwasm/query.ts voting-verifier-threshold <chain-name> -e <env>
```

---

#### Update Signing Threshold (MultisigProver)

**Action:** (SLA: 24h+ for governance)
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c MultisigProver \
  -n <chain-name> \
  -t "Update signing threshold for <chain>" \
  -d "Update signing threshold" \
  --deposit 100000000 \
  --msg '{"update_signing_threshold":{"new_signing_threshold":["<numerator>","<denominator>"]}}'
```

**Verification:**
```bash
ts-node cosmwasm/query.ts multisig-prover-signing-threshold <chain-name> -e <env>
```

---

### 3.4 ITS Hub Flow-Related Incident

**Trigger Threshold:** Volume spike > 3x average of past 5 hours

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Freeze affected chain | 15 min | No | Emergency Operator / Governance |
| 2 | Investigate flow metrics | 2h | Yes (with step 1) | Operations |
| 3 | Unfreeze after investigation | 30 min | No | Emergency Operator / Governance |

#### Freeze Chain on ITS Hub

**✅ Both Admin EOA and Governance can do this** (SLA: 15 min)

**Option A: Direct execution with Admin EOA (Recommended for emergency):**
```bash
ts-node cosmwasm/contract.ts its-freeze-chain <chain-name> -e <env> -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/contract.ts its-freeze-chain <chain-name> -e <env> --governance \
  -t "Freeze <chain-name> on ITS Hub" \
  -d "Freeze chain due to abnormal flows"
```

**Verification:**
```bash
ts-node cosmwasm/query.ts its-is-chain-frozen <chain-name> -e <env>
# Expected output: true
```

**Backup Action:** If freeze fails, disable execution entirely via `its-disable-execution`.

---

#### Unfreeze Chain on ITS Hub

**✅ Both Admin EOA and Governance can do this** (SLA: 30 min after investigation)

**Option A: Direct execution with Admin EOA:**
```bash
ts-node cosmwasm/contract.ts its-unfreeze-chain <chain-name> -e <env> -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/contract.ts its-unfreeze-chain <chain-name> -e <env> --governance \
  -t "Unfreeze <chain-name> on ITS Hub" \
  -d "Unfreeze chain after investigation"
```

**Verification:**
```bash
ts-node cosmwasm/query.ts its-is-chain-frozen <chain-name> -e <env>
# Expected output: false
```

---

#### Update Chain Configuration

**Action:**
```bash
ts-node cosmwasm/contract.ts its-hub-register-chains <chain-name> \
  --update \
  -t "Update <chain-name> configuration" \
  -d "Update chain configuration on ITS Hub" \
  --deposit 100000000
```

**Verification:**
```bash
ts-node cosmwasm/query.ts its-chain-config <chain-name>
```

---

### 3.5 Configuration / Deployment Error

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Query current config | 30 min | No | DevOps |
| 2 | Freeze/pause if unsafe | 15 min | Yes (with step 1) | Emergency Operator |
| 3 | Fix via governance | 24h+ | No | Governance |
| 4 | Verify fix | 30 min | No | DevOps |

**Step 1: Query current configuration**
```bash
ts-node cosmwasm/query.ts its-chain-config <chain-name>
ts-node cosmwasm/query.ts contract-info -c <ContractName>
ts-node cosmwasm/query.ts rewards <chain-name>
```

**Step 2: Freeze/pause if unsafe** (if needed, PARALLEL with Step 1)

Refer to Section 3.1 for freeze/disable commands.

**Step 3: Fix via governance proposal**
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c <ContractName> \
  -t "Fix configuration for <ContractName>" \
  -d "Correct misconfigured parameters" \
  --deposit 100000000 \
  --msg '<appropriate-fix-message>'
```

**Step 4: Verify fix**
```bash
ts-node cosmwasm/query.ts contract-info -c <ContractName>
```

---

## 4. Verification Commands Quick Reference

### Query Contract State

```bash
# Query ITS chain configuration
ts-node cosmwasm/query.ts its-chain-config <chain-name> -e <env>

# Query contract info (version, address)
ts-node cosmwasm/query.ts contract-info -c <ContractName> -e <env>

# Query all contract versions
ts-node cosmwasm/query.ts contract-versions -e <env>

# Query rewards pool state
ts-node cosmwasm/query.ts rewards <chain-name> -e <env>

# Query token configuration
ts-node cosmwasm/query.ts token-config <tokenId> -e <env>
```

### Emergency Status Queries

```bash
# Query Router frozen status
ts-node cosmwasm/query.ts router-is-chain-frozen <chain-name> -e <env>

# Query Multisig signing status
ts-node cosmwasm/query.ts multisig-is-signing-enabled -e <env>

# Query Multisig authorized callers
ts-node cosmwasm/query.ts multisig-authorized-caller <chain-name> -e <env>

# Query VotingVerifier voting threshold
ts-node cosmwasm/query.ts voting-verifier-threshold <chain-name> -e <env>

# Query MultisigProver signing threshold
ts-node cosmwasm/query.ts multisig-prover-signing-threshold <chain-name> -e <env>

# Query MultisigProver current verifier set
ts-node cosmwasm/query.ts multisig-prover-current-verifier-set <chain-name> -e <env>

# Query MultisigProver next verifier set
ts-node cosmwasm/query.ts multisig-prover-next-verifier-set <chain-name> -e <env>

# Query ITS Hub chain frozen status
ts-node cosmwasm/query.ts its-is-chain-frozen <chain-name> -e <env>

# Query ITS Hub execution status
ts-node cosmwasm/query.ts its-is-execution-enabled -e <env>

# Query contract admin
ts-node cosmwasm/query.ts contract-admin -c <ContractName> -e <env>
# For chain-specific contracts (VotingVerifier, MultisigProver, Gateway):
ts-node cosmwasm/query.ts contract-admin -c <ContractName> -n <chain-name> -e <env>
```

---

## 5. Backup Actions Summary

| Primary Action | Backup Action | Escalation |
|----------------|---------------|------------|
| Freeze chain on Router | Disable routing entirely | Contact validators to halt chain |
| Freeze chain on ITS Hub | Disable ITS execution entirely | Contact validators to halt chain |
| Disable signing on Multisig | Contact validators to halt chain | - |
| Governance proposal fails | Direct validator coordination | Emergency governance call |
| Update admin via governance | Emergency governance fast-track | Validator set intervention |

---

## 6. Post-Incident Checklist

For any significant emergency (particularly on mainnet), once the immediate issue is contained:

- **Technical**
  - [ ] Confirm all pause/freeze/disable states are in the intended final state.
  - [ ] Verify contract states using verification commands above.
  - [ ] Reconcile logs from ampd, chains, and monitoring systems.
  - [ ] Document all tx hashes and block heights.

- **Governance and Documentation**
  - [ ] Summarize root cause, impacted components, and on-chain actions.
  - [ ] Update any environment-specific runbooks and chain configs.
  - [ ] File incident report with timeline.

- **Communication**
  - [ ] Notify internal stakeholders within 1 hour.
  - [ ] For user-facing impact, publish a post-mortem within 48 hours.

- **Follow-up Improvements**
  - [ ] Add or tune alerts for abnormal behavior.
  - [ ] Review admin/governance role scopes.
  - [ ] Schedule a review of this playbook with lessons learned.
  - [ ] Update SLAs if needed.

---

## 7. Emergency Commands Quick Reference

### Router Operations (Admin EOA Only)

```bash
# Freeze a chain on Router
ts-node cosmwasm/contract.ts router-freeze-chain <chain-name> -e <env> -y

# Unfreeze a chain on Router
ts-node cosmwasm/contract.ts router-unfreeze-chain <chain-name> -e <env> -y

# Disable ALL routing (killswitch)
ts-node cosmwasm/contract.ts router-disable-routing -e <env> -y

# Enable routing
ts-node cosmwasm/contract.ts router-enable-routing -e <env> -y
```

### Multisig Operations (Admin EOA or Governance)

```bash
# Disable signing - Admin EOA
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> -y

# Disable signing - Governance
ts-node cosmwasm/contract.ts multisig-disable-signing -e <env> --governance \
  -t "Disable signing" -d "Emergency disable"

# Enable signing - Admin EOA
ts-node cosmwasm/contract.ts multisig-enable-signing -e <env> -y

# Enable signing - Governance
ts-node cosmwasm/contract.ts multisig-enable-signing -e <env> --governance \
  -t "Enable signing" -d "Re-enable after fix"
```

### ITS Hub Operations (Admin EOA or Governance)

```bash
# Disable execution - Admin EOA
ts-node cosmwasm/contract.ts its-disable-execution -e <env> -y

# Disable execution - Governance
ts-node cosmwasm/contract.ts its-disable-execution -e <env> --governance \
  -t "Disable ITS execution" -d "Emergency disable"

# Enable execution - Admin EOA
ts-node cosmwasm/contract.ts its-enable-execution -e <env> -y

# Enable execution - Governance
ts-node cosmwasm/contract.ts its-enable-execution -e <env> --governance \
  -t "Enable ITS execution" -d "Re-enable after fix"

# Freeze chain - Admin EOA
ts-node cosmwasm/contract.ts its-freeze-chain <chain-name> -e <env> -y

# Freeze chain - Governance
ts-node cosmwasm/contract.ts its-freeze-chain <chain-name> -e <env> --governance \
  -t "Freeze <chain-name>" -d "Freeze due to abnormal activity"

# Unfreeze chain - Admin EOA
ts-node cosmwasm/contract.ts its-unfreeze-chain <chain-name> -e <env> -y

# Unfreeze chain - Governance
ts-node cosmwasm/contract.ts its-unfreeze-chain <chain-name> -e <env> --governance \
  -t "Unfreeze <chain-name>" -d "Unfreeze after investigation"
```

### Governance-Only Operations (Use `submit-proposal.js execute`)

| Operation | Contract | Script Command |
|-----------|----------|---------------|
| `authorize_callers` | Multisig | `ts-node cosmwasm/submit-proposal.js execute -c Multisig --msg '{"authorize_callers":{...}}'` |
| `unauthorize_callers` | Multisig | `ts-node cosmwasm/submit-proposal.js execute -c Multisig --msg '{"unauthorize_callers":{...}}'` |
| `update_voting_threshold` | VotingVerifier | `ts-node cosmwasm/submit-proposal.js execute -c VotingVerifier -n <chain> --msg '{"update_voting_threshold":{...}}'` |
| `update_signing_threshold` | MultisigProver | `ts-node cosmwasm/submit-proposal.js execute -c MultisigProver -n <chain> --msg '{"update_signing_threshold":{...}}'` |
| `update_admin` | Various | `ts-node cosmwasm/submit-proposal.js execute -c <Contract> --msg '{"update_admin":{...}}'` |

### XRPL-Specific Operations

These currently require `ts-node cosmwasm/submit-proposal.js execute` with raw messages:

| Operation | Contract |
|-----------|----------|
| `disable_execution` / `enable_execution` | XrplVotingVerifier |
| `disable_execution` / `enable_execution` | XrplGateway |
| `trust_set` | XrplMultisigProver |

---

## 8. Quick Reference: Environment-Specific Addresses

| Environment | Governance Module | Config File |
|-------------|-------------------|-------------|
| mainnet | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar-chains-config/info/mainnet.json` |
| testnet | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar-chains-config/info/testnet.json` |
| stagenet | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar-chains-config/info/stagenet.json` |
| devnet-amplifier | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar-chains-config/info/devnet-amplifier.json` |

To get contract addresses for a specific environment:
```bash
# Read from config
cat axelar-chains-config/info/<env>.json | jq '.axelar.contracts'
```
