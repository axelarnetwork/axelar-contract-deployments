# Sui Chains Role Transfers & Cap Management

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                       |

| **Environment** | **Chain** | **Deployment Status** | **Date** |
| --------------- | --------- | --------------------- | -------- |
| **Devnet**      | `sui`     | -                     | TBD      |
| **Stagenet**    | `sui`     | -                     | TBD      |
| **Testnet**     | `sui`     | -                     | TBD      |
| **Mainnet**     | `sui`     | -                     | TBD      |

## Background

Rotate non‑critical roles to appropriate operational addresses, and assign critical roles to Multisig contract. This enforces correct permissions, separation of duties, and stronger security.

### Role Transfer Summary

| Contract | Role | Current Role Owner | Operations | Assign To | Reasoning |
|----------|------|----------------------|-----------|-----------|-----------|
| AxelarGateway | OwnerCap | EOA | `transfer_owner_cap`, `upgrade` | Multisig | CRITICAL PROTOCOL CONTROL |
| AxelarGateway | UpgradeCap | EOA | `upgrade` | Multisig | CRITICAL PROTOCOL UPGRADE |
| GasService | OwnerCap | EOA | `transfer_owner_cap`, `upgrade` | Multisig | CRITICAL PROTOCOL UPGRADE |
| GasService | UpgradeCap | EOA | `upgrade` | Multisig | CRITICAL PROTOCOL UPGRADE |
| GasService | OperatorCap | Operators contract | `collect_gas`, `refund` | Operators | TREASURY AND OPERATIONAL MANAGEMENT |
| Operators | OwnerCap | EOA | `add_operator`, `remove_operator` | Relayer Operators EOA | OPERATIONAL REGISTRY MANAGEMENT |
| Operators | UpgradeCap | EOA | `upgrade` | Relayer Operators EOA | OPERATIONAL REGISTRY MANAGEMENT |
| InterchainTokenService | OwnerCap | EOA | `add_trusted_chains`, `remove_trusted_chains` | Multisig | OPERATIONAL TOKEN SERVICE MANAGEMENT |
| InterchainTokenService | UpgradeCap | EOA | `upgrade` | Multisig | OPERATIONAL TOKEN SERVICE MANAGEMENT |
| InterchainTokenService | OperatorCap | EOA | `set_flow_limit` | Rate Limiter EOA | OPERATIONAL FLOW LIMIT MANAGEMENT |

## Pre-requisites

1. Sui CLI installed and configured for the target environment
   ```bash
   sui --version
   sui client envs
   ```
2. Create an `.env` config for local commands
   ```yaml
   PRIVATE_KEY=<deployer private key or configured keystore alias>
   ENV=<devnet|stagenet|testnet|mainnet>
   CHAIN=sui
   ```
3. Ensure the following are present in `axelar-chains-config/info/${ENV}.json`
   - `chains["sui"].contracts.AxelarGateway.{packageId, ownerCapId, upgradeCapId}`
   - `chains["sui"].contracts.GasService.{packageId, ownerCapId, upgradeCapId, operatorCapId}`
   - `chains["sui"].contracts.Operators.{packageId, ownerCapId, upgradeCapId, address}`
   - `chains["sui"].contracts.InterchainTokenService.{packageId, ownerCapId, upgradeCapId, operatorCapId}`
   - `chains["sui"].roles.{multisig, relayerOperatorsEOA, rateLimiterEOA}`
4. TODO: Ensure `multisig` contract is tested and scripts are functional
5. TODO: Confirm final `multisig` addresses per environment
6. TODO: Confirm `relayerOperatorsEOA` and `rateLimiterEOA` per environment
7. TODO: Confirm all Cap object IDs (OwnerCap/UpgradeCap/OperatorCap) and package IDs

## Deployment Steps

Notes:
- Some modules expose explicit `transfer_owner_cap` functions; others allow direct Sui object transfer of the `OwnerCap`/`UpgradeCap` objects. Use the approach your deployed package supports.
- Replace env var lookups with the correct keys if your JSON layout differs.

### Step 1: Transfer AxelarGateway OwnerCap to Multisig

| Network | Current OwnerCap Holder | Target Address |
| -------- | ----------------------- | -------------- |
| **Devnet** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | Multisig |
| **Stagenet** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | Multisig |
| **Testnet** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | Multisig |
| **Mainnet** | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | Multisig |

```bash
cd /Users/socrates/Axelar/axelar-contract-deployments
MULTISIG=$(jq -r '.chains["sui"].roles.multisig' ./axelar-chains-config/info/$ENV.json)
GATEWAY_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.AxelarGateway.ownerCapId' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName AxelarGateway --objectName OwnerCap --recipient "$MULTISIG"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GATEWAY_OWNERCAP_ID" --recipient "$MULTISIG"

# Verify
sui client object "$GATEWAY_OWNERCAP_ID"
```

### Step 2: Transfer AxelarGateway UpgradeCap to Multisig

```bash
UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.AxelarGateway.upgradeCapId' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName AxelarGateway --objectName UpgradeCap --recipient "$MULTISIG"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$UPG_CAP_ID" --recipient "$MULTISIG"

sui client object "$UPG_CAP_ID"
```

### Step 3: Transfer GasService OwnerCap to Multisig

```bash
GS_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.GasService.ownerCapId' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName GasService --objectName OwnerCap --recipient "$MULTISIG"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GS_OWNERCAP_ID" --recipient "$MULTISIG"

sui client object "$GS_OWNERCAP_ID"
```

### Step 4: Transfer GasService UpgradeCap to Multisig

```bash
GS_UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.GasService.upgradeCapId' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName GasService --objectName UpgradeCap --recipient "$MULTISIG"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GS_UPG_CAP_ID" --recipient "$MULTISIG"

sui client object "$GS_UPG_CAP_ID"
```

### Step 5: Transfer Operators OwnerCap to Relayer Operators EOA

```bash
OPERATORS_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.Operators.ownerCapId' ./axelar-chains-config/info/$ENV.json)
RELAYER_OPERATORS_EOA=$(jq -r '.chains["sui"].roles.relayerOperatorsEOA' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName Operators --objectName OwnerCap --recipient "$RELAYER_OPERATORS_EOA"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$OPERATORS_OWNERCAP_ID" --recipient "$RELAYER_OPERATORS_EOA"

sui client object "$OPERATORS_OWNERCAP_ID"
```

### Step 6: Transfer Operators UpgradeCap to Relayer Operators EOA

```bash
OPERATORS_UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.Operators.upgradeCapId' ./axelar-chains-config/info/$ENV.json)
RELAYER_OPERATORS_EOA=$(jq -r '.chains["sui"].roles.relayerOperatorsEOA' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName Operators --objectName UpgradeCap --recipient "$RELAYER_OPERATORS_EOA"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$OPERATORS_UPG_CAP_ID" --recipient "$RELAYER_OPERATORS_EOA"

sui client object "$OPERATORS_UPG_CAP_ID"
```

### Step 7: Transfer InterchainTokenService OwnerCap to Multisig

```bash
ITS_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.InterchainTokenService.ownerCapId' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OwnerCap --recipient "$MULTISIG"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_OWNERCAP_ID" --recipient "$MULTISIG"

sui client object "$ITS_OWNERCAP_ID"
```

### Step 8: Transfer InterchainTokenService UpgradeCap to Multisig

```bash
ITS_UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.InterchainTokenService.upgradeCapId' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName UpgradeCap --recipient "$MULTISIG"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_UPG_CAP_ID" --recipient "$MULTISIG"

sui client object "$ITS_UPG_CAP_ID"
```

### Step 9: Transfer InterchainTokenService OperatorCap to Rate Limiter EOA

```bash
ITS_OPERATORCAP_ID=$(jq -r '.chains["sui"].contracts.InterchainTokenService.operatorCapId' ./axelar-chains-config/info/$ENV.json)
RATE_LIMITER_EOA=$(jq -r '.chains["sui"].roles.rateLimiterEOA' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OperatorCap --recipient "$RATE_LIMITER_EOA"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_OPERATORCAP_ID" --recipient "$RATE_LIMITER_EOA"

# Verify
sui client object "$ITS_OPERATORCAP_ID"
```

## Verification Checklist

After completing cap transfers:

- [ ] AxelarGateway `OwnerCap` is held by Multisig
- [ ] AxelarGateway `UpgradeCap` is held by Multisig
- [ ] GasService `OwnerCap` is held by Multisig
- [ ] GasService `UpgradeCap` is held by Multisig
- [ ] Operators `OwnerCap` is held by Relayer Operators EOA
- [ ] Operators `UpgradeCap` is held by Relayer Operators EOA
- [ ] InterchainTokenService `OwnerCap` is held by Multisig
- [ ] InterchainTokenService `UpgradeCap` is held by Multisig
- [ ] InterchainTokenService `OperatorCap` is held by Rate Limiter EOA
- [ ] All transfers verified via `sui client object <cap_id>`
- [ ] Contract addresses and cap object IDs updated in `${ENV}.json`
- [ ] Documentation updated with new role addresses
