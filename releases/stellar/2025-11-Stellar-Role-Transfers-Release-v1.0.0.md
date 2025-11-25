# Stellar Chains Role Transfers

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @sean329 <sean.xu@interoplabs.io>   |
| **Deployment** |                                      |

| **Environment**          | **Chain**           | **Deployment Status** |
| ------------------------ | ------------------- | --------------------- |
| **Stagenet**             | `stellar-2025-q2`   | Deployed              |
| **Testnet**              | `stellar-2025-q2-2` | Deployed              |
| **Mainnet**              | `stellar`           | Deployed              |

## Background

Rotate non-critical roles to appropriate operational addresses, and assign critical roles to governance contracts or appropriate EOAs. This enforces correct permissions, separation of duties, and stronger security.

**Deployment Note:** Stellar testnet resets every quarter and we have stopped redeploying to devnet-amplifier. We only deploy on stagenet, testnet, and mainnet. This release document covers role transfers for these three environments only.

### Role Transfer Summary

| Contract                  | Role     | Current Role Owner | Operations                                                                          | Assign To                | Reasoning                                                                                                                                                                         |
|---------------------------|----------|--------------------|-------------------------------------------------------------------------------------|--------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| AxelarGateway             | owner    | EOA                | `transfer_ownership`, `pause/unpause`, `upgrade`                                    | AxelarServiceGovernance  | CRITICAL PROTOCOL CONTROL - Owner has pause/unpause duties which need rapid responses; operator cannot do pause/unpause. This is where Stellar differs from EVM                  |
| AxelarGateway             | operator | EOA                | `rotate_signers`, `transfer_operatorship`                                           | Emergency Operator EOA   | EMERGENCY RESPONSE - Compromised signers need to be rotated out in rapid response                                                                                                 |
| AxelarOperators           | owner    | EOA                | `add_operator`, `remove_operator`, `transfer_ownership`, `upgrade`                  | Relayer Operators EOA    | OPERATIONAL REGISTRY MANAGEMENT - Common owner duties, aligned with EVM settings                                                                                                  |
| AxelarGasService          | owner    | EOA                | `transfer_ownership`, `upgrade`                                                     | AxelarServiceGovernance  | CRITICAL PROTOCOL CONTROL - Common owner duties, aligned with EVM settings                                                                                                        |
| AxelarGasService          | operator | Operators contract | `collect_fees`, `refund`, `transfer_operatorship`                                   | Operators                | TREASURY AND OPERATIONAL MANAGEMENT - GasService owner cannot add operators, so the only operator role needs to be the Operators contract where more EOA operators can be added |
| InterchainTokenService    | owner    | EOA                | `transfer_token_admin`, `transfer_ownership`, `pause/unpause`, `upgrade`            | AxelarServiceGovernance  | CRITICAL PROTOCOL CONTROL - Owner has pause/unpause duties which need rapid responses; operator cannot do pause/unpause                                                           |
| InterchainTokenService    | operator | EOA                | `set_trusted_chain`, `remove_trusted_chain`, `set_flow_limit`, `transfer_operatorship` | Rate Limiter EOA      | OPERATIONAL MANAGEMENT - Update rate limits and trusted chains                                                                                                                    |

**Notes:** 
- Contracts like `Upgrader`, `Multicall`, and `TokenUtils` do not have transferable roles. The deployer field is informational only and requires no action.
- **Important:** The `pause/unpause` functions in `AxelarGateway` and `InterchainTokenService` are currently owner-only operations. These functions will be migrated to operator-accessible operations in a future contract upgrade. This role transfer document reflects the current state where owners retain pause/unpause capabilities.

## Prerequisites

Create an `.env` config. `CHAIN` should be set to `stellar` for mainnet, and the appropriate Stellar chain identifier for other networks.

```yaml
PRIVATE_KEY=<stellar_deployer_key>
ENV=<stagenet|testnet|mainnet>
CHAIN=<stellar-2025-q2|stellar-2025-q2-2|stellar>
```

Ensure you have the necessary permissions and have verified the current role owners before proceeding.

## Current Role Owners

**Note:** Stellar testnet resets every quarter and we have stopped redeploying to devnet. We only deploy on stagenet and testnet. Therefore, only stagenet and testnet current role owners are listed below.

### Stagenet / Testnet

| Contract               | Role     | Current Address                                      |
| ---------------------- | -------- | ---------------------------------------------------- |
| AxelarGateway          | owner    | `GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3` (S) <br> `GA6HQ5Z4O6T3MFYDC4MIJSYOIGSX2HYMZ5V6DI3NRBNIL7JX7A7IEO5Z` (T) |
| AxelarGateway          | operator | `GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3` (S) <br> `GA6HQ5Z4O6T3MFYDC4MIJSYOIGSX2HYMZ5V6DI3NRBNIL7JX7A7IEO5Z` (T) |
| AxelarOperators        | owner    | `GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3` (S) <br> `GA6HQ5Z4O6T3MFYDC4MIJSYOIGSX2HYMZ5V6DI3NRBNIL7JX7A7IEO5Z` (T) |
| AxelarGasService       | owner    | `GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3` (S) <br> `GA6HQ5Z4O6T3MFYDC4MIJSYOIGSX2HYMZ5V6DI3NRBNIL7JX7A7IEO5Z` (T) |
| AxelarGasService       | operator | `CBYTWO3NS3BFVSZSNJL7BH2REK7V2OCKV5HMLISPP4PLGAX3NEPIPYD3` (S) <br> `CDV2FWNSYYKRVJ3HLXTKGF3XYYU5X5CXCC72CKNRLW2CXDYU4PX3F72E` (T) |
| InterchainTokenService | owner    | `GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3` (S) <br> `GA6HQ5Z4O6T3MFYDC4MIJSYOIGSX2HYMZ5V6DI3NRBNIL7JX7A7IEO5Z` (T) |
| InterchainTokenService | operator | `GBP4FSAOFV5O72AB3YQRDCYVD47W4N7KQK3OJODXSU3OBPNGKX4SQTJ3` (S) <br> `GA6HQ5Z4O6T3MFYDC4MIJSYOIGSX2HYMZ5V6DI3NRBNIL7JX7A7IEO5Z` (T) |

### Mainnet

| Contract               | Role     | Current Address                                      |
| ---------------------- | -------- | ---------------------------------------------------- |
| AxelarGateway          | owner    | `GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3` |
| AxelarGateway          | operator | `GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3` |
| AxelarOperators        | owner    | `GCUIBOS2JPTJSJ3PFMXU4RD67PS5QT7FG3HSXHFZQGVNIYXPYODKRJ7S` |
| AxelarGasService       | owner    | `GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3` |
| AxelarGasService       | operator | `CCO23C66LAPU5YO66VNXB75T7SDVZ5UZ2GHAU3M7T2YGRKHJI3B2LZPQ` |
| InterchainTokenService | owner    | `GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3` |
| InterchainTokenService | operator | `GC2SJ4YXCMP2LYXMXBNJMK6SNK4XUR7TGJXY4GA3VACNMCZVCQ6VFGG3` |

## Target Role Addresses

Before executing the role transfers, confirm the target addresses for each environment:

| Role Target               | Network          | Address    |
| ------------------------- | ---------------- | ---------- |
| AxelarServiceGovernance   | Stagenet         | TBD        |
| AxelarServiceGovernance   | Testnet          | TBD        |
| AxelarServiceGovernance   | Mainnet          | TBD        |
| Emergency Operator EOA    | Stagenet         | TBD        |
| Emergency Operator EOA    | Testnet          | TBD        |
| Emergency Operator EOA    | Mainnet          | TBD        |
| Relayer Operators EOA     | Stagenet         | TBD        |
| Relayer Operators EOA     | Testnet          | TBD        |
| Relayer Operators EOA     | Mainnet          | TBD        |
| Rate Limiter EOA          | Stagenet         | TBD        |
| Rate Limiter EOA          | Testnet          | TBD        |
| Rate Limiter EOA          | Mainnet          | TBD        |

## Deployment Steps

### Step 1: Verify Current Role Owners

Before making any transfers, verify the current owners and operators for all contracts:

```bash
# Verify AxelarGateway
ts-node stellar/contract.js owner AxelarGateway
ts-node stellar/contract.js operator AxelarGateway

# Verify AxelarOperators
ts-node stellar/contract.js owner AxelarOperators

# Verify AxelarGasService
ts-node stellar/contract.js owner AxelarGasService
ts-node stellar/contract.js operator AxelarGasService

# Verify InterchainTokenService
ts-node stellar/contract.js owner InterchainTokenService
ts-node stellar/contract.js operator InterchainTokenService
```

### Step 2: Transfer AxelarGateway Owner to AxelarServiceGovernance

**New Owner**: AxelarServiceGovernance

| Network      | Target Address |
| ------------ | -------------- |
| **Stagenet** | TBD            |
| **Testnet**  | TBD            |
| **Mainnet**  | TBD            |

```bash
AXELAR_SERVICE_GOVERNANCE=<AXELAR_SERVICE_GOVERNANCE_ADDRESS>

# Transfer ownership
ts-node stellar/contract.js transfer-ownership AxelarGateway $AXELAR_SERVICE_GOVERNANCE

# Verify transfer
ts-node stellar/contract.js owner AxelarGateway
```

### Step 3: Transfer AxelarGateway Operator to Emergency Operator EOA

**New Operator**: Emergency Operator EOA

| Network      | Target Address |
| ------------ | -------------- |
| **Stagenet** | TBD            |
| **Testnet**  | TBD            |
| **Mainnet**  | TBD            |

```bash
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>

# Transfer operatorship
ts-node stellar/contract.js transfer-operatorship AxelarGateway $EMERGENCY_OPERATOR_EOA

# Verify transfer
ts-node stellar/contract.js operator AxelarGateway
```

### Step 4: Transfer AxelarOperators Owner to Relayer Operators EOA

**New Owner**: Relayer Operators EOA

| Network      | Target Address |
| ------------ | -------------- |
| **Stagenet** | TBD            |
| **Testnet**  | TBD            |
| **Mainnet**  | TBD            |

```bash
RELAYER_OPERATORS_EOA=<RELAYER_OPERATORS_EOA_ADDRESS>

# Transfer ownership
ts-node stellar/contract.js transfer-ownership AxelarOperators $RELAYER_OPERATORS_EOA

# Verify transfer
ts-node stellar/contract.js owner AxelarOperators
```

### Step 5: Transfer AxelarGasService Owner to AxelarServiceGovernance

**New Owner**: AxelarServiceGovernance

| Network      | Target Address |
| ------------ | -------------- |
| **Stagenet** | TBD            |
| **Testnet**  | TBD            |
| **Mainnet**  | TBD            |

```bash
AXELAR_SERVICE_GOVERNANCE=<AXELAR_SERVICE_GOVERNANCE_ADDRESS>

# Transfer ownership
ts-node stellar/contract.js transfer-ownership AxelarGasService $AXELAR_SERVICE_GOVERNANCE

# Verify transfer
ts-node stellar/contract.js owner AxelarGasService
```

### Step 6: Verify AxelarGasService Operator (Already Operators Contract)

The `AxelarGasService` operator should already be set to the `Operators` contract. Verify this is the case:

```bash
# Verify operator
ts-node stellar/contract.js operator AxelarGasService
```

**Expected Operator Addresses:**

| Network      | Operators Contract Address                               |
| ------------ | -------------------------------------------------------- |
| **Stagenet** | `CBRMCHA6EEVQJVKIBDLOXGZSOPUXMYXYMKPNVNNFCDBIP7VEFQCHBLXR` |
| **Testnet**  | `CCZEQG2QFL3WV2GPXO4BRCVIEZBMRJXAHJQWCUMNPQIXNMAD4NPZBF3M` |
| **Mainnet**  | `CCO23C66LAPU5YO66VNXB75T7SDVZ5UZ2GHAU3M7T2YGRKHJI3B2LZPQ` |

If the operator is NOT the Operators contract, transfer it:

```bash
OPERATORS_CONTRACT=$(jq -r '.chains[$CHAIN].contracts.AxelarOperators.address' ./axelar-chains-config/info/$ENV.json)

# Transfer operatorship if needed
ts-node stellar/contract.js transfer-operatorship AxelarGasService $OPERATORS_CONTRACT

# Verify transfer
ts-node stellar/contract.js operator AxelarGasService
```

### Step 7: Transfer InterchainTokenService Owner to AxelarServiceGovernance

**New Owner**: AxelarServiceGovernance

| Network      | Target Address |
| ------------ | -------------- |
| **Stagenet** | TBD            |
| **Testnet**  | TBD            |
| **Mainnet**  | TBD            |

```bash
AXELAR_SERVICE_GOVERNANCE=<AXELAR_SERVICE_GOVERNANCE_ADDRESS>

# Transfer ownership
ts-node stellar/contract.js transfer-ownership InterchainTokenService $AXELAR_SERVICE_GOVERNANCE

# Verify transfer
ts-node stellar/contract.js owner InterchainTokenService
```

### Step 8: Transfer InterchainTokenService Operator to Rate Limiter EOA

**New Operator**: Rate Limiter EOA

| Network      | Target Address |
| ------------ | -------------- |
| **Stagenet** | TBD            |
| **Testnet**  | TBD            |
| **Mainnet**  | TBD            |

```bash
RATE_LIMITER_EOA=<RATE_LIMITER_EOA_ADDRESS>

# Transfer operatorship
ts-node stellar/contract.js transfer-operatorship InterchainTokenService $RATE_LIMITER_EOA

# Verify transfer
ts-node stellar/contract.js operator InterchainTokenService
```

## Verification Checklist

After completing role transfers, verify all changes:

- [ ] AxelarGateway `owner` is held by AxelarServiceGovernance
- [ ] AxelarGateway `operator` is held by Emergency Operator EOA
- [ ] AxelarOperators `owner` is held by Relayer Operators EOA
- [ ] AxelarGasService `owner` is held by AxelarServiceGovernance
- [ ] AxelarGasService `operator` is held by Operators contract
- [ ] InterchainTokenService `owner` is held by AxelarServiceGovernance
- [ ] InterchainTokenService `operator` is held by Rate Limiter EOA
- [ ] All transfers verified via `ts-node stellar/contract.js owner/operator <contractName>`
- [ ] Contract addresses updated in `${ENV}.json` if necessary
- [ ] Documentation updated with new role addresses

## Notes

1. **Stellar Role Model**: Stellar contracts have a different role model compared to EVM chains. The `owner` role in Stellar contracts currently includes `pause/unpause` capabilities that cannot be delegated to operators, which is why some roles are assigned to governance rather than following the exact EVM pattern.

2. **Future Contract Upgrade**: The `pause/unpause` functions in `AxelarGateway` and `InterchainTokenService` will be migrated to operator-accessible operations in an upcoming contract upgrade. Once this upgrade is completed, these emergency functions will be more readily accessible for rapid response scenarios without requiring governance action.

3. **GasService Operator**: The `AxelarGasService` operator must be the `Operators` contract because the owner of `GasService` cannot add individual operator accounts. Additional EOA operators can be added through the `AxelarOperators` contract.

4. **Emergency Response**: The `Emergency Operator EOA` for `AxelarGateway` is designated for rapid response scenarios where compromised signers need to be rotated quickly.

5. **No Action Required**: Contracts like `Upgrader`, `Multicall`, and `TokenUtils` are utility contracts without transferable ownership roles. The `deployer` field in these contracts is informational only.

6. **Governance Contracts**: Ensure that the AxelarServiceGovernance contract addresses are deployed and operational in each environment before transferring ownership to them.

7. **Multi-Signature Requirements**: Depending on the governance contract implementation, some operations may require multi-signature approval. Coordinate with the appropriate signers before executing transfers.

