# Sui Chains Role Transfers & Cap Management

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                        |

| **Environment**      | **Chain** | **Deployment Status** | **Date** |
| -------------------- | --------- | --------------------- | -------- |
| **Devnet Amplifier** | `sui-2`   | -                     | TBD      |
| **Stagenet**         | `sui`     | -                     | TBD      |
| **Testnet**          | `sui`     | -                     | TBD      |
| **Mainnet**          | `sui`     | -                     | TBD      |

## Background

Rotate non‑critical roles to appropriate operational addresses, and assign critical roles to EOA (ultimatly will be tranferred to Multisig/ServiceGovernance). This enforces correct permissions, separation of duties, and stronger security. 


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
    - `chains["sui"].contracts.AxelarGateway.{address, objects: {OwnerCap, UpgradeCap}}`
    - `chains["sui"].contracts.GasService.{address,  objects: {OwnerCap, UpgradeCap, OperatorCap}}`
    - `chains["sui"].contracts.Operators.{address,  objects: {OwnerCap, UpgradeCap}}`
    - `chains["sui"].contracts.InterchainTokenService.{address,  objects: {OwnerCap, UpgradeCap, OperatorCap}}`

## Deployment Steps

Notes:

- Some modules expose explicit `transfer_owner_cap` functions; others will require directly transferring the `OwnerCap`/`UpgradeCap` objects to the recipient. Use the exposed transfer function if your deployed package supports it.
- Replace env var lookups with the correct keys if your JSON layout differs.

### Step 1: Transfer AxelarGateway OwnerCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x4a755c3a0d51e25f64542e8f39ec7db91ca7cc194b5aec7efb77d54c2b67ffee` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
# Set EOA address from the table above
TARGET_ADDRESS=
GATEWAY_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.AxelarGateway.objects.OwnerCap' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName AxelarGateway --objectName OwnerCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GATEWAY_OWNERCAP_ID" --recipient "$TARGET_ADDRESS"

# Verify
sui client object "$"
```

### Step 2: Transfer AxelarGateway UpgradeCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x4a755c3a0d51e25f64542e8f39ec7db91ca7cc194b5aec7efb77d54c2b67ffee` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.AxelarGateway.objects.UpgradeCap' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName AxelarGateway --objectName UpgradeCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$UPG_CAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$UPG_CAP_ID"
```

### Step 3: Transfer GasService OwnerCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x4a755c3a0d51e25f64542e8f39ec7db91ca7cc194b5aec7efb77d54c2b67ffee` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
GS_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.GasService.objects.OwnerCap' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName GasService --objectName OwnerCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GS_OWNERCAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$GS_OWNERCAP_ID"
```

### Step 4: Transfer GasService UpgradeCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x4a755c3a0d51e25f64542e8f39ec7db91ca7cc194b5aec7efb77d54c2b67ffee` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
GS_UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.GasService.objects.UpgradeCap' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName GasService --objectName UpgradeCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GS_UPG_CAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$GS_UPG_CAP_ID"
```

### Step 5(Optional): Transfer Operators OwnerCap to Relayer Operators EOA 

**New Owner**: Relayer Operators EOA

| Network              | Current Owner                                                        | Target Address |
| -------------------- | -------------------------------------------------------------------- | -------------- |
| **Devnet Amplifier** | `0x619592640cab068848c92c309bdd665d6a5e3f2f2f51ec9464cc112166daf7d1` | TBD            |
| **Stagenet**         | `0x619592640cab068848c92c309bdd665d6a5e3f2f2f51ec9464cc112166daf7d1` | TBD            |
| **Testnet**          | `0x619592640cab068848c92c309bdd665d6a5e3f2f2f51ec9464cc112166daf7d1` | TBD            |
| **Mainnet**          | `0xd7b392db51562a72e50f310e78c827b4e917254cf15c5cec6c97964299a6be2a` | TBD            |

```bash
OPERATORS_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.Operators.objects.OwnerCap' ./axelar-chains-config/info/$ENV.json)
RELAYER_OPERATORS_EOA=<TARGET_ADDRESS>

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName Operators --objectName OwnerCap --recipient "$RELAYER_OPERATORS_EOA"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$OPERATORS_OWNERCAP_ID" --recipient "$RELAYER_OPERATORS_EOA"

sui client object "$OPERATORS_OWNERCAP_ID"
```

### Step 6: Transfer InterchainTokenService OwnerCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x4a755c3a0d51e25f64542e8f39ec7db91ca7cc194b5aec7efb77d54c2b67ffee` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
ITS_OWNERCAP_ID=$(jq -r '.chains["sui"].contracts.InterchainTokenService.objects.OwnerCap' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OwnerCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_OWNERCAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$ITS_OWNERCAP_ID"
```

### Step 7: Transfer InterchainTokenService UpgradeCap to new EOA

| Network             | Current OwnerCap Holder                                              | Target Address                                                       |
| ------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amlifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**        | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x4a755c3a0d51e25f64542e8f39ec7db91ca7cc194b5aec7efb77d54c2b67ffee` |
| **Testnet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**         | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
ITS_UPG_CAP_ID=$(jq -r '.chains["sui"].contracts.InterchainTokenService.objects.UpgradeCap' ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName UpgradeCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_UPG_CAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$ITS_UPG_CAP_ID"
```

### Step 8: Transfer InterchainTokenService OperatorCap to Rate Limiter EOA

**New Operator**: Rate Limiter EOA

| Network              | Current Operator                                                     | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x1471a8acf730a05a7d720e52c7ef94024c7351502c83b80da5583db2f6b0b8df` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xfcddd4a96f494e264cebf18f2c69527b36f0493bbec06e43f290323da0e1a2b8` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | TBD                                                                  |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
ITS_OPERATORCAP_ID=$(jq -r '.chains["sui"].contracts.InterchainTokenService.objects.OperatorCap' ./axelar-chains-config/info/$ENV.json)
RATE_LIMITER_EOA=<TARGET_ADDRESS>

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OperatorCap --recipient "$RATE_LIMITER_EOA"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_OPERATORCAP_ID" --recipient "$RATE_LIMITER_EOA"

# Verify
sui client object "$ITS_OPERATORCAP_ID"
```

## Verification Checklist

After completing cap transfers:

- [ ] AxelarGateway `OwnerCap` is held by EOA
- [ ] AxelarGateway `UpgradeCap` is held by EOA
- [ ] GasService `OwnerCap` is held by EOA
- [ ] GasService `UpgradeCap` is held by EOA
- [ ] Operators `OwnerCap` is held by Relayer Operators EOA
- [ ] InterchainTokenService `OwnerCap` is held by EOA
- [ ] InterchainTokenService `UpgradeCap` is held by EOA
- [ ] InterchainTokenService `OperatorCap` is held by Rate Limiter EOA
- [ ] All transfers verified via `sui client object <cap_id>`
- [ ] Contract addresses and cap object IDs updated in `${ENV}.json`
- [ ] Documentation updated with new role addresses
