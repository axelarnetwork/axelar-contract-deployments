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
axelard tx wasm execute <ITS_HUB_ADDRESS> \
  '{"disable_execution":{}}' \
  --from <its-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Option B: Via Governance proposal (if Admin key unavailable):**
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c InterchainTokenService \
  -t "Emergency: Disable ITS execution" \
  -d "Disable execution due to suspected exploit" \
  --deposit 100000000 \
  --msg '{"disable_execution":{}}'
```

**Verification:**
```bash
axelard q wasm contract-state smart <ITS_HUB_ADDRESS> '{"is_execution_enabled":{}}'
# Expected output: {"data":false}
```

**Backup Action:** If governance proposal fails, contact validators to halt the chain.

---

#### Step 2: Freeze specific chain on Router (if chain-specific) [PARALLEL with Step 1]

**⚠️ Requires Admin EOA** - Cannot use governance proposal

**Action (Direct execution with admin EOA):**
```bash
# Using axelard CLI with admin private key
axelard tx wasm execute <ROUTER_ADDRESS> \
  '{"freeze_chain":{"chain":"<chain-name>"}}' \
  --from <router-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q wasm contract-state smart <ROUTER_ADDRESS> '{"is_chain_frozen":{"chain":"<chain-name>"}}'
# Expected output: {"data":true}
```

**Backup Action:** If freeze fails, disable routing entirely via `disable_routing`.

---

#### Step 3: Disable signing on Multisig [PARALLEL with Step 1,2]

**✅ Both Admin EOA and Governance can do this** - Use Admin for faster response

**Option A: Direct execution with Admin EOA (Recommended for emergency):**
```bash
axelard tx wasm execute <MULTISIG_ADDRESS> \
  '{"disable_signing":{}}' \
  --from <multisig-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Emergency: Disable signing" \
  -d "Disable signing due to suspected exploit" \
  --deposit 100000000 \
  --msg '{"disable_signing":{}}'
```

**Verification:**
```bash
axelard q wasm contract-state smart <MULTISIG_ADDRESS> '{"is_signing_enabled":{}}'
# Expected output: {"data":false}
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
axelard tx wasm execute <ITS_HUB_ADDRESS> \
  '{"enable_execution":{}}' \
  --from <its-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c InterchainTokenService \
  -t "Re-enable ITS execution" \
  -d "Re-enable execution after fix deployed" \
  --deposit 100000000 \
  --msg '{"enable_execution":{}}'
```

**Verification:**
```bash
axelard q wasm contract-state smart <ITS_HUB_ADDRESS> '{"is_execution_enabled":{}}'
# Expected output: {"data":true}
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
ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Emergency: Disable signing" \
  -d "Contain compromised admin" \
  --deposit 100000000 \
  --msg '{"disable_signing":{}}'

# Option B: If using a different trusted admin EOA
axelard tx wasm execute <MULTISIG_ADDRESS> \
  '{"disable_signing":{}}' \
  --from <trusted-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q wasm contract-state smart <MULTISIG_ADDRESS> '{"is_signing_enabled":{}}'
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
axelard q wasm contract-state smart <CONTRACT_ADDRESS> '{"admin":{}}'
# Confirm new admin address
```

**Backup Action:** If governance is too slow, contact validators to halt the chain.

---

#### Key Rotation EOA (MultisigProver Admin) Compromised

**Step 1: Disable signing** (SLA: 15 min) [PARALLEL with Step 2]

**✅ Both Admin EOA and Governance can do this** - Use Admin for faster response
```bash
# Option A: Direct execution with Admin EOA (faster)
axelard tx wasm execute <MULTISIG_ADDRESS> \
  '{"disable_signing":{}}' \
  --from <multisig-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y

# Option B: Via Governance proposal
ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Emergency: Disable signing" \
  -d "Disable signing due to compromised key rotation EOA" \
  --deposit 100000000 \
  --msg '{"disable_signing":{}}'
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
axelard q wasm contract-state smart <MULTISIG_PROVER_ADDRESS> '{"admin":{}}'
```

**Step 3: Re-enable signing** (SLA: 30 min after Step 2)

**✅ Both Admin EOA and Governance can do this**
```bash
# Option A: Direct execution with Admin EOA
axelard tx wasm execute <MULTISIG_ADDRESS> \
  '{"enable_signing":{}}' \
  --from <multisig-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y

# Option B: Via Governance proposal
ts-node cosmwasm/submit-proposal.js execute \
  -c Multisig \
  -t "Re-enable signing" \
  -d "Re-enable signing after admin rotation" \
  --deposit 100000000 \
  --msg '{"enable_signing":{}}'
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
ts-node cosmwasm/rotate-signers.js update-verifier-set <chain-name>
```

**Verification:**
```bash
axelard q wasm contract-state smart <MULTISIG_PROVER_ADDRESS> '{"next_verifier_set":{}}'
```

**Step 3: Submit proof on destination chain** (SLA: 30 min)
- For Sui: `ts-node sui/gateway.js submitProof <multisig-session-id>`
- For EVM: `ts-node evm/gateway.js --action submitProof --multisigSessionId <multisig-session-id> -n <chain-name>`

**Step 4: Confirm verifier rotation** (SLA: 30 min)
```bash
ts-node cosmwasm/rotate-signers.js confirm-verifier-rotation <chain-name> <rotate-signers-tx>
```

**Verification:**
```bash
axelard q wasm contract-state smart <MULTISIG_PROVER_ADDRESS> '{"current_verifier_set":{}}'
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
axelard q wasm contract-state smart <VOTING_VERIFIER_ADDRESS> '{"voting_threshold":{}}'
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
axelard q wasm contract-state smart <MULTISIG_PROVER_ADDRESS> '{"signing_threshold":{}}'
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
axelard tx wasm execute <ITS_HUB_ADDRESS> \
  '{"freeze_chain":{"chain":"<chain-name>"}}' \
  --from <its-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c InterchainTokenService \
  -t "Freeze <chain-name> on ITS Hub" \
  -d "Freeze chain due to abnormal flows" \
  --deposit 100000000 \
  --msg '{"freeze_chain":{"chain":"<chain-name>"}}'
```

**Verification:**
```bash
axelard q wasm contract-state smart <ITS_HUB_ADDRESS> '{"is_chain_frozen":{"chain":"<chain-name>"}}'
# Expected output: {"data":true}
```

**Backup Action:** If freeze fails, disable execution entirely via `disable_execution`.

---

#### Unfreeze Chain on ITS Hub

**✅ Both Admin EOA and Governance can do this** (SLA: 30 min after investigation)

**Option A: Direct execution with Admin EOA:**
```bash
axelard tx wasm execute <ITS_HUB_ADDRESS> \
  '{"unfreeze_chain":{"chain":"<chain-name>"}}' \
  --from <its-admin-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id <chain-id> \
  --node <rpc-url> \
  -y
```

**Option B: Via Governance proposal:**
```bash
ts-node cosmwasm/submit-proposal.js execute \
  -c InterchainTokenService \
  -t "Unfreeze <chain-name> on ITS Hub" \
  -d "Unfreeze chain after investigation" \
  --deposit 100000000 \
  --msg '{"unfreeze_chain":{"chain":"<chain-name>"}}'
```

**Verification:**
```bash
axelard q wasm contract-state smart <ITS_HUB_ADDRESS> '{"is_chain_frozen":{"chain":"<chain-name>"}}'
# Expected output: {"data":false}
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
ts-node cosmwasm/query.ts its-chain-config <chain-name>

# Query contract info (version, address)
ts-node cosmwasm/query.ts contract-info -c <ContractName>

# Query all contract versions
ts-node cosmwasm/query.ts contract-versions -e <env>

# Query rewards pool state
ts-node cosmwasm/query.ts rewards <chain-name>

# Query token configuration
ts-node cosmwasm/query.ts token-config <tokenId>
```

### Query via axelard CLI

```bash
# Query Router frozen status
axelard q wasm contract-state smart <ROUTER_ADDRESS> '{"is_chain_frozen":{"chain":"<chain-name>"}}'

# Query Multisig signing status
axelard q wasm contract-state smart <MULTISIG_ADDRESS> '{"is_signing_enabled":{}}'

# Query Multisig authorized callers
axelard q wasm contract-state smart <MULTISIG_ADDRESS> '{"authorized_caller":{"chain_name":"<chain-name>"}}'

# Query VotingVerifier voting threshold
axelard q wasm contract-state smart <VOTING_VERIFIER_ADDRESS> '{"voting_threshold":{}}'

# Query MultisigProver signing threshold
axelard q wasm contract-state smart <MULTISIG_PROVER_ADDRESS> '{"signing_threshold":{}}'

# Query ITS Hub chain status
axelard q wasm contract-state smart <ITS_HUB_ADDRESS> '{"its_chain":{"chain":"<chain-name>"}}'

# Query ITS Hub execution status
axelard q wasm contract-state smart <ITS_HUB_ADDRESS> '{"is_execution_enabled":{}}'

# Query contract admin
axelard q wasm contract-state smart <CONTRACT_ADDRESS> '{"admin":{}}'
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

## 7. Missing Script Support (Action Items)

### Summary: Which Operations Can Use `submit-proposal.js execute`?

Based on the [Role Transfers document](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1217/files):

### ❌ Admin-Only Operations (CANNOT use governance proposal)

These operations can ONLY be executed by Admin EOA. Use `axelard tx wasm execute` directly:

| Operation | Contract | Message Format |
|-----------|----------|----------------|
| `freeze_chain` | Router | `{"freeze_chain":{"chain":"<chain>"}}` |
| `unfreeze_chain` | Router | `{"unfreeze_chain":{"chain":"<chain>"}}` |
| `disable_routing` | Router | `{"disable_routing":{}}` |
| `enable_routing` | Router | `{"enable_routing":{}}` |
| `register_p2p_token_instance` | InterchainTokenService | `{"register_p2p_token_instance":{...}}` |
| `modify_supply` | InterchainTokenService | `{"modify_supply":{...}}` |

**Example for Admin-Only operation:**
```bash
axelard tx wasm execute <ROUTER_ADDRESS> \
  '{"freeze_chain":{"chain":"<chain-name>"}}' \
  --from <router-admin-key> \
  --gas auto --gas-adjustment 1.3 --gas-prices 0.00005uaxl \
  --chain-id <chain-id> --node <rpc-url> -y
```

### ✅ Both Admin & Governance Operations (CAN use either)

These operations can be done by Admin EOA (faster) OR via governance proposal:

| Operation | Contract | Message Format |
|-----------|----------|----------------|
| `disable_signing` | Multisig | `{"disable_signing":{}}` |
| `enable_signing` | Multisig | `{"enable_signing":{}}` |
| `unauthorize_callers` | Multisig | `{"unauthorize_callers":{"contracts":{...}}}` |
| `freeze_chain` | InterchainTokenService | `{"freeze_chain":{"chain":"<chain>"}}` |
| `unfreeze_chain` | InterchainTokenService | `{"unfreeze_chain":{"chain":"<chain>"}}` |
| `disable_execution` | InterchainTokenService | `{"disable_execution":{}}` |
| `enable_execution` | InterchainTokenService | `{"enable_execution":{}}` |
| `update_verifier_set` | MultisigProver | via `rotate-signers.js` or proposal |

### ✅ Governance-Only Operations (Use `submit-proposal.js execute`)

### High Priority (Emergency Operations)

| Operation | Contract | Message Format |
|-----------|----------|---------------|
| `freeze_chain` | Router | `{"freeze_chain":{"chain":"<chain>"}}` |
| `unfreeze_chain` | Router | `{"unfreeze_chain":{"chain":"<chain>"}}` |
| `disable_routing` | Router | `{"disable_routing":{}}` |
| `enable_routing` | Router | `{"enable_routing":{}}` |
| `disable_signing` | Multisig | `{"disable_signing":{}}` |
| `enable_signing` | Multisig | `{"enable_signing":{}}` |
| `authorize_callers` | Multisig | `{"authorize_callers":{"contracts":{"<chain>":"<prover_addr>"}}}` |
| `unauthorize_callers` | Multisig | `{"unauthorize_callers":{"contracts":{"<chain>":"<prover_addr>"}}}` |
| `freeze_chain` | InterchainTokenService | `{"freeze_chain":{"chain":"<chain>"}}` |
| `unfreeze_chain` | InterchainTokenService | `{"unfreeze_chain":{"chain":"<chain>"}}` |
| `disable_execution` | InterchainTokenService | `{"disable_execution":{}}` |
| `enable_execution` | InterchainTokenService | `{"enable_execution":{}}` |

### Medium Priority (Threshold/Admin Updates)

| Operation | Contract | Message Format |
|-----------|----------|---------------|
| `update_voting_threshold` | VotingVerifier | `{"update_voting_threshold":{"new_voting_threshold":["n","d"]}}` |
| `update_signing_threshold` | MultisigProver | `{"update_signing_threshold":{"new_signing_threshold":["n","d"]}}` |
| `update_admin` | MultisigProver | `{"update_admin":{"new_admin":"<addr>"}}` |
| `update_admin` | Router | `{"update_admin":{"new_admin":"<addr>"}}` |
| `update_admin` | Multisig | `{"update_admin":{"new_admin":"<addr>"}}` |

### Lower Priority (Service Management)

| Operation | Contract | Message Format |
|-----------|----------|---------------|
| `authorize_verifiers` | ServiceRegistry | `{"authorize_verifiers":{"verifiers":[...],"service_name":"..."}}` |
| `unauthorize_verifiers` | ServiceRegistry | `{"unauthorize_verifiers":{"verifiers":[...],"service_name":"..."}}` |
| `jail_verifiers` | ServiceRegistry | `{"jail_verifiers":{"verifiers":[...],"service_name":"..."}}` |
| `update_pool_params` | Rewards | `{"update_pool_params":{"pool_id":{...},"params":{...}}}` |

### XRPL-Specific Operations

| Operation | Contract | Message Format |
|-----------|----------|---------------|
| `disable_execution` | XrplVotingVerifier | `{"disable_execution":{}}` |
| `enable_execution` | XrplVotingVerifier | `{"enable_execution":{}}` |
| `disable_execution` | XrplGateway | `{"disable_execution":{}}` |
| `enable_execution` | XrplGateway | `{"enable_execution":{}}` |
| `trust_set` | XrplMultisigProver | `{"trust_set":{"issuer":"...","currency":"..."}}` |

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

---

## Comments / Open Items

**General comment**: Each step needs to have its SLA marked; also if some steps are
determined can go parallel rather than sequential, it should be labeled.
✅ **Addressed**: Added SLA and parallelization tables for each playbook.

**General comment**: Missing escalation path for each action, for example, the
decision of pausing ITS should obtain whose approval? How about changing flow
limit, freeze token…etc., can the decision be made by any engineer holding the
Rate Limiter EOA role alone, or must also obtain X's approval?
✅ **Addressed**: Added Role Reference & Escalation Path section with approval requirements.

**General comment**: Need to add verification instructions after each action step,
for example, after pausing the ITS, how to verify the status has become paused?
✅ **Addressed**: Added verification commands after each action step.

**General comment**: Each action A ideally needs a backup action B in case for some
reason A cannot be executed successfully. For example, if wanting to limit the
flow or freeze some token but unsuccessful, then pausing the ITS can be its back
up action; and if need to pause the ITS but unsuccessful, then contacting
validators to pause the chain is its backup action?
✅ **Addressed**: Added backup actions for each critical step and a summary table.

**Need to define a rule of thumb trigger value** - what's the abnormal volume?
Something like a spike 3x of the average volume of the past 5 hours?
✅ **Addressed**: Added trigger threshold definition (3x average of past 5 hours).

**For Axelar Amplifier**: Unlike EVM ITS which moved pause/unpause to operators, 
most Axelar Amplifier contracts still require governance for critical operations.
Consider moving emergency operations (freeze/unfreeze, disable/enable) to admin
roles for faster response time.
⚠️ **TODO**: Create PR to enable admin-based rapid response for emergency operations.

**Open Questions / Action Items**:
1. ⚠️ **HIGH PRIORITY**: Implement dedicated CLI commands for admin emergency operations in `contract.ts`
   - Current gap: Admin operations (freeze, disable_signing, etc.) cannot use `submit-proposal.js`
   - Need: Add `--admin` flag or separate commands that execute directly with admin EOA
2. What is the exact threshold for "abnormal volume" per chain/token type?
3. Should we create a dedicated `emergency.ts` script for rapid response operations?
4. Should admin keys be stored in HSM/secure enclave for emergency access?
5. Document the exact admin EOA addresses for each environment in a secure location.
