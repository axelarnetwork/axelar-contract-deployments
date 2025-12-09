# Axelar Consensus Chain Emergency Response Playbook

## 1. Emergency Categories

- **Chain Security Incident**
  - Critical bug found in EVM/Axelarnet/Nexus modules.
  - Suspicious cross-chain activity suggesting an exploit.

- **Key / Role Compromise**
  - Compromise or suspected compromise of:
    - Governance multisig key (ROLE_ACCESS_CONTROL)
    - Chain management account (ROLE_CHAIN_MANAGEMENT)
    - Validator proxy accounts

- **Cross-Chain Transfer Related Incident**
  - Abnormal transfer volumes on specific chains.
  - Need to halt transfers without fully stopping the network.
  - **Trigger threshold**: Volume spike > 3x average of past 5 hours

- **Validator / Signing Related Incident**
  - Compromised validator key needing rotation.
  - Key rotation for EVM gateway contracts.
  - Threshold adjustments needed.

- **Configuration / Deployment Error**
  - Misconfigured chain parameters or gateway addresses.
  - Incorrect fee configurations.

---

## 2. Escalation Path & Decision Authority

### Who Can Approve Emergency Actions?

| Action Category | Engineer Can Do Alone? | Requires Leadership Approval? | Notes |
|-----------------|------------------------|------------------------------|-------|
| **Deactivate single chain** | ✅ Yes | No | Rapid response needed, can be reversed |
| **Activate chain** | ⚠️ After verification | Recommended | Confirm threat is resolved first |
| **Deactivate all chains** (`:all:`) | ❌ No | ✅ Required | Affects ALL chains, major impact |
| **Disable link-deposit** | ✅ Yes | No | Emergency containment for deposit protocol |
| **Enable link-deposit** | ⚠️ After verification | Recommended | Confirm threat is resolved first |
| **Set transfer rate limit** | ❌ No | ✅ Required | Affects cross-chain transfers |
| **Rotate key** | ❌ No | ✅ Required | Security-critical change |
| **Update params** | ❌ No | ✅ Required | Security-critical change |
| **Register/Deregister controller** | ❌ No | ✅ Required | Permission management |

### Permission Roles Reference

| Role | Description | Typical Holder |
|------|-------------|----------------|
| **ROLE_ACCESS_CONTROL** | Highest privilege, can perform critical operations | Governance multisig |
| **ROLE_CHAIN_MANAGEMENT** | Chain operations, key rotation, params updates | Chain management account |
| **ROLE_UNRESTRICTED** | Normal operations (register maintainer, submit signatures) | Any account |

### Escalation Contacts

| Severity | When to Escalate | Contact | Response Time SLA |
|----------|------------------|---------|-------------------|
| **P0 - Critical** | Active exploit, funds at risk | Security Team Lead + CTO | 15 min |
| **P1 - High** | Key compromise, all-chain deactivation | Security Team Lead | 30 min |
| **P2 - Medium** | Single chain deactivation, abnormal flow | Engineering Lead | 1 hour |
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
│  (deactivate chain, disable link-deposit)                       │
└─────────────────────────────────────────────────────────────────┘
          │                                    │
         YES                                  NO
          │                                    │
          ▼                                    ▼
┌─────────────────────┐          ┌─────────────────────────────────┐
│ Engineer executes   │          │ Escalate to Leadership          │
│ immediately         │          │ (deactivate :all:, rate limits, │
│ (notify team after) │          │  key rotation, etc.)            │
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

### Module & Permission Reference

| Module | Operations | Required Role |
|--------|------------|---------------|
| **nexus** | ActivateChain, DeactivateChain | ROLE_ACCESS_CONTROL |
| **nexus** | SetTransferRateLimit | ROLE_ACCESS_CONTROL |
| **nexus** | EnableLinkDeposit, DisableLinkDeposit | ROLE_ACCESS_CONTROL |
| **nexus** | RegisterAssetFee | ROLE_CHAIN_MANAGEMENT |
| **evm** | SetGateway, AddChain | ROLE_ACCESS_CONTROL |
| **evm** | CreateDeployToken, CreateTransferOperatorship | ROLE_CHAIN_MANAGEMENT |
| **axelarnet** | AddCosmosBasedChain, RegisterFeeCollector | ROLE_ACCESS_CONTROL |
| **axelarnet** | RegisterAsset | ROLE_CHAIN_MANAGEMENT |
| **multisig** | StartKeygen, RotateKey | ROLE_CHAIN_MANAGEMENT |
| **permission** | RegisterController, DeregisterController | ROLE_ACCESS_CONTROL |

---

## 3. Response Playbooks by Incident Type

### 3.1 Chain Security Incident / Exploit Risk

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Deactivate affected chain(s) | 15 min | No | ROLE_ACCESS_CONTROL |
| 2 | Disable link-deposit protocol | 15 min | Yes (with step 1) | ROLE_ACCESS_CONTROL |
| 3 | Set transfer rate limit to 0 | 15 min | Yes (with step 1,2) | ROLE_ACCESS_CONTROL |
| 4 | Investigate and develop fix | 24-72h | No | Engineering |
| 5 | Deploy fix (if code change needed) | 24h+ | No | Governance |
| 6 | Re-enable operations | 30 min | No | ROLE_ACCESS_CONTROL |

#### Step 1: Deactivate Affected Chain(s)

**⚠️ Requires ROLE_ACCESS_CONTROL permission**

**Action (Deactivate a single chain):**
```bash
axelard tx nexus deactivate-chain ethereum \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Action (Deactivate ALL chains - requires leadership approval):**
```bash
axelard tx nexus deactivate-chain :all: \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
# Query chain state
axelard q nexus chain-state ethereum
# Expected: activated: false

# Or query all chains
axelard q nexus chains
```

**Backup Action:** If deactivation fails, contact validators to halt the chain.

---

#### Step 2: Disable Link-Deposit Protocol [PARALLEL with Step 1]

**⚠️ Requires ROLE_ACCESS_CONTROL permission**

**Action:**
```bash
axelard tx nexus disable-link-deposit \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q nexus link-deposit-enabled
# Expected: enabled: false
```

**Note:** This disables Link, ConfirmDeposit, and CreateBurnTokens operations across all chains.

---

#### Step 3: Set Transfer Rate Limit to Zero [PARALLEL with Step 1,2]

**⚠️ Requires ROLE_ACCESS_CONTROL permission**

**Action:**
```bash
# Set rate limit to 0 for a specific asset on a chain
axelard tx nexus set-transfer-rate-limit ethereum 0uaxl 1h \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q nexus transfer-rate-limit ethereum uaxl
# Check the limit is set to 0
```

---

#### Step 4: Investigate and Develop Fix

**Action:** Validate the mitigation on devnet/testnet before mainnet.

---

#### Step 5: Deploy Fix (if needed)

**Action:** For consensus chain code changes, a network upgrade is required:
1. Prepare upgrade proposal
2. Submit governance proposal
3. Validators upgrade after proposal passes

---

#### Step 6: Re-enable Operations

**⚠️ Requires ROLE_ACCESS_CONTROL permission**

**Activate chain:**
```bash
axelard tx nexus activate-chain ethereum \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Enable link-deposit:**
```bash
axelard tx nexus enable-link-deposit \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Restore rate limit:**
```bash
axelard tx nexus set-transfer-rate-limit ethereum 1000000uaxl 24h \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q nexus chain-state ethereum
# Expected: activated: true

axelard q nexus link-deposit-enabled
# Expected: enabled: true
```

---

### 3.2 Key / Role Compromise

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Contain blast radius (deactivate chains) | 15 min | No | ROLE_ACCESS_CONTROL |
| 2 | Coordinate to ignore compromised account | 30 min | Yes (with step 1) | Security Team |
| 3 | Deregister compromised controller | 24h+ | No | ROLE_ACCESS_CONTROL |
| 4 | Register new controller | 24h+ | No | ROLE_ACCESS_CONTROL |
| 5 | Restore operations | 30 min | No | ROLE_ACCESS_CONTROL |

#### Governance Key (ROLE_ACCESS_CONTROL) Compromised

**Step 1: Contain** (SLA: 15 min)

Deactivate all chains to prevent further damage:
```bash
axelard tx nexus deactivate-chain :all: \
  --from <backup-access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Step 2: Coordinate** (SLA: 30 min)
1. Notify all signers to ignore proposals not officially announced.
2. If a malicious proposal already has signatures, coordinate to not reach threshold.

**Step 3: Update Governance Key via Governance Proposal** (SLA: 24h+)

This requires a governance proposal to update the governance key:
```bash
# This is typically done through a governance proposal
# The exact process depends on your governance setup
```

**Verification:**
```bash
axelard q permission governance-key
# Confirm new governance key is set
```

---

#### Chain Management (ROLE_CHAIN_MANAGEMENT) Compromised

**Step 1: Deregister Compromised Controller** (SLA: 24h+)

**⚠️ Requires ROLE_ACCESS_CONTROL permission**
```bash
axelard tx permission deregister-controller <compromised-address> \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Step 2: Register New Controller** (SLA: 24h+)

**⚠️ Requires ROLE_ACCESS_CONTROL permission**
```bash
axelard tx permission register-controller <new-address> \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q permission params
# Verify controller addresses
```

---

### 3.3 Key Rotation / Signing Related Incident

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Start new keygen | 1h | No | ROLE_CHAIN_MANAGEMENT |
| 2 | Validators submit public keys | 1h | No | Validators |
| 3 | Rotate to new key | 1h | No | ROLE_CHAIN_MANAGEMENT |
| 4 | Transfer operatorship on destination | 30 min | No | ROLE_CHAIN_MANAGEMENT |

#### Rotate Key for a Chain

**Step 1: Start Keygen** (SLA: 1h)

**⚠️ Requires ROLE_CHAIN_MANAGEMENT permission**
```bash
axelard tx multisig start-keygen <new-key-id> \
  --from <chain-management-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q multisig key <new-key-id>
# Check key state
```

**Step 2: Rotate Key** (SLA: 1h after keygen completes)

**⚠️ Requires ROLE_CHAIN_MANAGEMENT permission**
```bash
axelard tx multisig rotate-key ethereum <new-key-id> \
  --from <chain-management-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Step 3: Transfer Operatorship on EVM Chain** (SLA: 30 min)

**⚠️ Requires ROLE_CHAIN_MANAGEMENT permission**
```bash
axelard tx evm transfer-operatorship ethereum <new-key-id> \
  --from <chain-management-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Step 4: Sign and Execute Transfer**
```bash
axelard tx evm sign-commands ethereum \
  --from <any-account> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q multisig key-id ethereum
# Verify new key ID is active

axelard q evm batched-commands ethereum <batch-id>
# Verify command batch status
```

---

### 3.4 Cross-Chain Transfer Related Incident

**Trigger Threshold:** Volume spike > 3x average of past 5 hours

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Deactivate affected chain | 15 min | No | ROLE_ACCESS_CONTROL |
| 2 | Set transfer rate limit | 15 min | Yes (with step 1) | ROLE_ACCESS_CONTROL |
| 3 | Investigate flow metrics | 2h | Yes (with step 1,2) | Operations |
| 4 | Reactivate after investigation | 30 min | No | ROLE_ACCESS_CONTROL |

#### Deactivate Chain

**⚠️ Requires ROLE_ACCESS_CONTROL permission** (SLA: 15 min)

```bash
axelard tx nexus deactivate-chain ethereum \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q nexus chain-state ethereum
# Expected: activated: false
```

---

#### Set Transfer Rate Limit

**⚠️ Requires ROLE_ACCESS_CONTROL permission** (SLA: 15 min)

```bash
# Limit transfers to 1000 tokens per hour
axelard tx nexus set-transfer-rate-limit ethereum 1000000000uusdc 1h \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q nexus transfer-rate-limit ethereum uusdc
```

---

#### Reactivate Chain After Investigation

**⚠️ Requires ROLE_ACCESS_CONTROL permission** (SLA: 30 min after investigation)

```bash
axelard tx nexus activate-chain ethereum \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Verification:**
```bash
axelard q nexus chain-state ethereum
# Expected: activated: true
```

---

### 3.5 Configuration / Deployment Error

| Step | Action | SLA | Can Parallelize | Responsible Role |
|------|--------|-----|-----------------|------------------|
| 1 | Query current config | 30 min | No | DevOps |
| 2 | Deactivate if unsafe | 15 min | Yes (with step 1) | ROLE_ACCESS_CONTROL |
| 3 | Fix configuration | 24h+ | No | ROLE_CHAIN_MANAGEMENT / Governance |
| 4 | Verify fix | 30 min | No | DevOps |

**Step 1: Query current configuration**
```bash
# Query chain state
axelard q nexus chain-state <chain-name>

# Query all chains
axelard q nexus chains

# Query EVM chain params
axelard q evm params <chain-name>

# Query gateway address
axelard q evm gateway-address <chain-name>

# Query fee info
axelard q nexus fee <chain-name> <asset>
```

**Step 2: Deactivate if unsafe** (if needed, PARALLEL with Step 1)

Refer to Section 3.1 for deactivate commands.

**Step 3: Fix configuration**

**Register Asset Fee:**
```bash
axelard tx nexus register-asset-fee ethereum uusdc 0.001 1000000 100000000 \
  --from <chain-management-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Set Gateway Address:**
```bash
axelard tx evm set-gateway ethereum 0x1234567890abcdef1234567890abcdef12345678 \
  --from <access-control-key> \
  --gas auto \
  --gas-adjustment 1.3 \
  --gas-prices 0.00005uaxl \
  --chain-id axelar-dojo-1 \
  --node <rpc-url> \
  -y
```

**Step 4: Verify fix**
```bash
axelard q nexus fee <chain-name> <asset>
axelard q evm gateway-address <chain-name>
```

---

## 4. Verification Commands Quick Reference

### Query Chain State

```bash
# Query specific chain state
axelard q nexus chain-state <chain-name>

# Query all chains
axelard q nexus chains

# Query if chain is activated
axelard q nexus chain-state <chain-name> | grep activated
```

### Query Link-Deposit Status

```bash
# Query if link-deposit is enabled
axelard q nexus link-deposit-enabled
```

### Query Transfer Rate Limits

```bash
# Query transfer rate limit for an asset on a chain
axelard q nexus transfer-rate-limit <chain-name> <denom>

# Query all rate limits
axelard q nexus transfer-rate-limits
```

### Query Key Information

```bash
# Query current key ID for a chain
axelard q multisig key-id <chain-name>

# Query key details
axelard q multisig key <key-id>

# Query next key ID
axelard q multisig next-key-id <chain-name>
```

### Query EVM Chain Information

```bash
# Query gateway address
axelard q evm gateway-address <chain-name>

# Query EVM chain params
axelard q evm params <chain-name>

# Query batched commands status
axelard q evm batched-commands <chain-name> <batch-id>

# Query pending commands
axelard q evm pending-commands <chain-name>
```

### Query Permission Information

```bash
# Query governance key
axelard q permission governance-key

# Query all permission params
axelard q permission params
```

### Query Fee Information

```bash
# Query fee info for an asset
axelard q nexus fee <chain-name> <asset>
```

---

## 5. Backup Actions Summary

| Primary Action | Backup Action | Escalation |
|----------------|---------------|------------|
| Deactivate single chain | Deactivate all chains (`:all:`) | Contact validators to halt chain |
| Disable link-deposit | Deactivate all chains | Contact validators to halt chain |
| Set rate limit to 0 | Deactivate chain | Contact validators to halt chain |
| Deregister controller | Emergency governance proposal | Validator set intervention |

---

## 6. Post-Incident Checklist

For any significant emergency (particularly on mainnet), once the immediate issue is contained:

- **Technical**
  - [ ] Confirm all chain states are in the intended final state.
  - [ ] Verify chain activation status using verification commands above.
  - [ ] Reconcile logs from relayers and monitoring systems.
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
  - [ ] Review permission role assignments.
  - [ ] Schedule a review of this playbook with lessons learned.
  - [ ] Update SLAs if needed.


