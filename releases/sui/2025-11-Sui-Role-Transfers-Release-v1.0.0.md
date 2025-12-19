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
    SIGNATURE_SCHEME=secp256k1
    ```
    *NOTE: `CHAIN=sui-2` for devnet-amplifier

3. Ensure the following are present in `axelar-chains-config/info/${ENV}.json`
    - `chains["$CHAIN"].contracts.AxelarGateway.{address, objects: {OwnerCap, UpgradeCap}}`
    - `chains["$CHAIN"].contracts.GasService.{address,  objects: {OwnerCap, UpgradeCap, OperatorCap}}`
    - `chains["$CHAIN"].contracts.Operators.{address,  objects: {OwnerCap, UpgradeCap}}`
    - `chains["$CHAIN"].contracts.InterchainTokenService.{address,  objects: {OwnerCap, UpgradeCap, OperatorCap}}`

## Deployment Steps

Notes:

- Some modules expose explicit `transfer_owner_cap` functions; others will require directly transferring the `OwnerCap`/`UpgradeCap` objects to the recipient. Use the exposed transfer function if your deployed package supports it.
- Replace env var lookups with the correct keys if your JSON layout differs.

### Step 1: Transfer AxelarGateway OwnerCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xd25e71a4726ca4da6561be45c2b7c1c2bb58e31db66c27bc48e5a7a4176d5d20` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x710889a7891ac29bf5bcd892fb8c0039dff68fdee98bb734eb8e6e34d3896105` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x0e11b06bb58020e868d01602f71f862153003217e22e974043eec302b0d68b24` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
# Set EOA address from the table above
TARGET_ADDRESS=
GATEWAY_OWNERCAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.AxelarGateway.objects.OwnerCap" ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName AxelarGateway --objectName OwnerCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GATEWAY_OWNERCAP_ID" --recipient "$TARGET_ADDRESS"

# Verify
sui client object "$GATEWAY_OWNERCAP_ID"
```

### Step 2: Transfer AxelarGateway UpgradeCap to new EOA

| Network              | Current UpgradeCap Holder                                            | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xd25e71a4726ca4da6561be45c2b7c1c2bb58e31db66c27bc48e5a7a4176d5d20` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x710889a7891ac29bf5bcd892fb8c0039dff68fdee98bb734eb8e6e34d3896105` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x0e11b06bb58020e868d01602f71f862153003217e22e974043eec302b0d68b24` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
UPG_CAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.AxelarGateway.objects.UpgradeCap" ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName AxelarGateway --objectName UpgradeCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$UPG_CAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$UPG_CAP_ID"
```

### Step 3: Transfer GasService OwnerCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xd25e71a4726ca4da6561be45c2b7c1c2bb58e31db66c27bc48e5a7a4176d5d20` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x710889a7891ac29bf5bcd892fb8c0039dff68fdee98bb734eb8e6e34d3896105` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x0e11b06bb58020e868d01602f71f862153003217e22e974043eec302b0d68b24` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
GS_OWNERCAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.GasService.objects.OwnerCap" ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName GasService --objectName OwnerCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GS_OWNERCAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$GS_OWNERCAP_ID"
```

### Step 4: Transfer GasService UpgradeCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xd25e71a4726ca4da6561be45c2b7c1c2bb58e31db66c27bc48e5a7a4176d5d20` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x710889a7891ac29bf5bcd892fb8c0039dff68fdee98bb734eb8e6e34d3896105` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x0e11b06bb58020e868d01602f71f862153003217e22e974043eec302b0d68b24` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
GS_UPG_CAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.GasService.objects.UpgradeCap" ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName GasService --objectName UpgradeCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$GS_UPG_CAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$GS_UPG_CAP_ID"
```

### Step 5(Optional): Transfer Operators OwnerCap to Relayer Operators EOA 

**New Owner**: Relayer Operators EOA

| Network              | Current Owner                                                        | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x619592640cab068848c92c309bdd665d6a5e3f2f2f51ec9464cc112166daf7d1` | `0x48ca40991593423094c3dfeca67e0e18cc43c498062be1b1277fba9042517601` |
| **Stagenet**         | `0x619592640cab068848c92c309bdd665d6a5e3f2f2f51ec9464cc112166daf7d1` | `0xb6a2934bc528ec3bcc6377f41f57058c98d0fb9412834502c0063596cd3f0ed3` |
| **Testnet**          | `0x619592640cab068848c92c309bdd665d6a5e3f2f2f51ec9464cc112166daf7d1` | `0x6b8d9a26087808166e217f0bf2d2fa580ee439d7fac21d54b7d12e60f123334c` |
| **Mainnet**          | `0xd7b392db51562a72e50f310e78c827b4e917254cf15c5cec6c97964299a6be2a` | TBD                                                                  |

```bash
OPERATORS_OWNERCAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.Operators.objects.OwnerCap" ./axelar-chains-config/info/$ENV.json)
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
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xd25e71a4726ca4da6561be45c2b7c1c2bb58e31db66c27bc48e5a7a4176d5d20` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x710889a7891ac29bf5bcd892fb8c0039dff68fdee98bb734eb8e6e34d3896105` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x0e11b06bb58020e868d01602f71f862153003217e22e974043eec302b0d68b24` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
ITS_OWNERCAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.InterchainTokenService.objects.OwnerCap" ./axelar-chains-config/info/$ENV.json)

# Using helper script by contract/object names
ts-node sui/transfer-object.js --contractName InterchainTokenService --objectName OwnerCap --recipient "$TARGET_ADDRESS"

# Or using direct object id
ts-node sui/transfer-object.js --objectId "$ITS_OWNERCAP_ID" --recipient "$TARGET_ADDRESS"

sui client object "$ITS_OWNERCAP_ID"
```

### Step 7: Transfer InterchainTokenService UpgradeCap to new EOA

| Network              | Current OwnerCap Holder                                              | Target Address                                                       |
| -------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- |
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xd25e71a4726ca4da6561be45c2b7c1c2bb58e31db66c27bc48e5a7a4176d5d20` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x710889a7891ac29bf5bcd892fb8c0039dff68fdee98bb734eb8e6e34d3896105` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x0e11b06bb58020e868d01602f71f862153003217e22e974043eec302b0d68b24` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
ITS_UPG_CAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.InterchainTokenService.objects.UpgradeCap" ./axelar-chains-config/info/$ENV.json)

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
| **Devnet Amplifier** | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x238d4ec927454b4e90a7257e249cced4f57578c42d37d250d67cbbf38b750db4` |
| **Stagenet**         | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0xa8b407ae179641e2a179a31479b3d788979f81235a6bbc7b655a13b68a26e742` |
| **Testnet**          | `0x3a6ff6c3d2b12d8acd39d9bbddca1094c28081123e59ffd0dee618d36207ee88` | `0x00dcfb27894ff7069d5f70d632629a74f33fa750b1e6758c19fdfbe4291f777a` |
| **Mainnet**          | `0x980372415053fe9d09956dea38d33d295f10de3d5c5226099304fe346ce241c9` | TBD                                                                  |

```bash
ITS_OPERATORCAP_ID=$(jq -r ".chains[\"$CHAIN\"].contracts.InterchainTokenService.objects.OperatorCap" ./axelar-chains-config/info/$ENV.json)
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
