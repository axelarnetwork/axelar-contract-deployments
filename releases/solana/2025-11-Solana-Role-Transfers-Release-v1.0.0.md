# Solana Chains Role Transfers

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @sean329 <sean.xu@interoplabs.io> |
| **Deployment** |                                    |

| **Environment**      | **Chain**       | **Deployment Status** |
| -------------------- | --------------- | --------------------- |
| **Devnet Amplifier** | `solana-devnet` | Deployed              |

## Background

Rotate non-critical roles to appropriate operational addresses, and assign critical roles to governance contracts or appropriate EOAs. This enforces correct permissions, separation of duties, and stronger security.

**Deployment Note:** Solana contracts are currently only deployed on **Devnet Amplifier**. This release document covers role transfers for the devnet-amplifier environment only. Future deployments to testnet and mainnet will follow a similar process.

**Governance Stability Note:** The Governance contract on Solana is not yet stable. Therefore, many UpgradeAuthority roles are temporarily assigned to **Relayer Operators EOA** instead of the Governance contract. Once the Governance contract is stable, these roles should be transferred to the Governance program.

### Role Transfer Summary

| Program                | Role                        | Current Role Owner                    | Operations                                                                                                                       | Assign To                     | Reasoning                                                                                    |
| ---------------------- | --------------------------- | ------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------- |
| Operators              | UpgradeAuthority            | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | Upgrade program                                                                                                                  | Relayer Operators EOA         | TEMPORARY - Governance contract not yet stable                                                |
| Operators              | Owner                       | Master Operator (configured at init)  | Add/remove operators, transfer ownership                                                                                         | Relayer Operators EOA         | OPERATIONAL REGISTRY MANAGEMENT                                                              |
| Governance             | UpgradeAuthority            | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | Upgrade program, configure contract                                                                                              | Relayer Operators EOA         | TEMPORARY - Governance contract not yet stable                                                |
| Governance             | Operator                    | Operators Wallet                      | Execute operator proposals, transfer operatorship, withdraw from contract, change configs                                        | Relayer Operators EOA         | OPERATIONAL GOVERNANCE MANAGEMENT                                                            |
| Memo                   | UpgradeAuthority            | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | Upgrade program                                                                                                                  | Relayer Operators EOA         | TEMPORARY - Governance contract not yet stable                                                |
| Gateway                | UpgradeAuthority            | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | Upgrade program, transfer operatorship                                                                                           | Relayer Operators EOA         | TEMPORARY - Governance contract not yet stable                                                |
| Gateway                | Operator                    | Operators Wallet                      | `rotate_signers` - disable rotation delay                                                                                        | Emergency Operator EOA        | EMERGENCY RESPONSE - Compromised signers need to be rotated quickly                          |
| ITS                    | UpgradeAuthority            | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | Upgrade program, initialize contract                                                                                             | Relayer Operators EOA         | TEMPORARY - Governance contract not yet stable                                                |
| ITS                    | Operator                    | Operators Wallet                      | `set_trusted_chain`, `remove_trusted_chain`, `set_flow_limit`, `set_pause_status`, `propose_operatorship`, `transfer_operatorship` | Rate Limiter EOA           | OPERATIONAL MANAGEMENT - set_pause_status needs rapid response                               |
| ITS (TokenManager)     | TokenManager Operator       | N/A (Token Project)                   | `set_token_manager_flow_limit`, `add_token_manager_flow_limiter`, `remove_token_manager_flow_limiter`, `propose_token_manager_operatorship`, `transfer_token_manager_operatorship` | Token Project Owner | TOKEN-SPECIFIC MANAGEMENT - Each token project manages their own token manager              |
| ITS (TokenManager)     | Minter                      | N/A (Token Project)                   | Mint tokens                                                                                                                      | Token Project Owner           | TOKEN-SPECIFIC MINTING                                                                       |
| ITS (TokenManager)     | Flow Limiter                | N/A (Token Project)                   | Manage flow limits for token                                                                                                     | Token Project Owner           | TOKEN-SPECIFIC FLOW CONTROL                                                                  |
| Gas Service            | UpgradeAuthority            | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | Upgrade program                                                                                                                  | Relayer Operators EOA         | TEMPORARY - Governance contract not yet stable                                                |
| Gas Service            | Operator (PDA)              | Operators Program                     | Create gas service treasury, collect fees, refund fees                                                                           | Operators Program             | TREASURY AND OPERATIONAL MANAGEMENT - Operator is a PDA controlled by Operators Program      |

**Notes:**
- **UpgradeAuthority Roles**: All UpgradeAuthority roles are temporarily assigned to **Relayer Operators EOA** because the Governance contract is not yet stable. Once Governance is production-ready, these roles should be transferred to the Governance program.
- **Token Project Roles**: TokenManager Operator, Minter, and Flow Limiter roles are not managed by Axelar. These are owned and managed by individual token projects.
- **Gas Service Operator PDA**: The Gas Service Operator is a Program Derived Address (PDA) controlled by the Operators Program, not a regular account.
- **Funder and Upgrade Authority Wallets**: These are utility wallets used for funding operations and do not have transferable roles.

## Prerequisites

Set up your environment for Solana operations:

```bash
# Set environment variables
export CHAIN=solana-devnet
export CLUSTER=devnet  # or use full RPC URL

# Ensure you have the solana-axelar-cli built
cd solana
cargo build --release
```

A Solana chain config should exist under `devnet-amplifier.json` file under the `chains` key.

## Current Role Owners

### Devnet Amplifier

| Program        | Role                  | Current Address                              |
| -------------- | --------------------- | -------------------------------------------- |
| Operators      | UpgradeAuthority      | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` |
| Operators      | Owner                 | Master Operator (configured at initialization) |
| Governance     | UpgradeAuthority      | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` |
| Governance     | Operator              | Operators Wallet                             |
| Memo           | UpgradeAuthority      | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` |
| Gateway        | UpgradeAuthority      | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` |
| Gateway        | Operator              | Operators Wallet                             |
| ITS            | UpgradeAuthority      | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` |
| ITS            | Operator              | Operators Wallet                             |
| Gas Service    | UpgradeAuthority      | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` |
| Gas Service    | Operator (PDA)        | Operators Program                            |

## Target Role Addresses

Before executing the role transfers, confirm the target addresses for devnet-amplifier:

| Role Target            | Network              | Address |
| ---------------------- | -------------------- | ------- |
| Relayer Operators EOA  | **Devnet Amplifier** | TBD     |
| Emergency Operator EOA | **Devnet Amplifier** | TBD     |
| Rate Limiter EOA       | **Devnet Amplifier** | TBD     |

## Deployment Steps

### Step 1: Verify Current Role Owners

Before making any transfers, verify the current upgrade authorities and operators. For Solana programs, you can query the upgrade authority using:

```bash
# Query Gateway upgrade authority and operator
solana program show <GATEWAY_PROGRAM_ID>

# Query ITS upgrade authority
solana program show <ITS_PROGRAM_ID>

# Query Governance upgrade authority
solana program show <GOVERNANCE_PROGRAM_ID>

# Query Operators upgrade authority
solana program show <OPERATORS_PROGRAM_ID>

# Query Gas Service upgrade authority
solana program show <GAS_SERVICE_PROGRAM_ID>

# Query Memo upgrade authority
solana program show <MEMO_PROGRAM_ID>
```

### Step 2: Transfer Gateway Operator to Emergency Operator EOA

**New Operator**: Emergency Operator EOA

| Network              | Target Address |
| -------------------- | -------------- |
| **Devnet Amplifier** | TBD            |

```bash
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>
CURRENT_OPERATOR=<CURRENT_OPERATOR_ADDRESS>

# Transfer Gateway operatorship
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  gateway transfer-operatorship \
  --authority $CURRENT_OPERATOR \
  --new-operator $EMERGENCY_OPERATOR_EOA \
  send

# Verify the transfer by checking Gateway config
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  query gateway config
```

### Step 3: Transfer ITS Operator to Rate Limiter EOA

**New Operator**: Rate Limiter EOA

| Network              | Target Address |
| -------------------- | -------------- |
| **Devnet Amplifier** | TBD            |

```bash
RATE_LIMITER_EOA=<RATE_LIMITER_EOA_ADDRESS>
CURRENT_ITS_OPERATOR=<CURRENT_ITS_OPERATOR_ADDRESS>

# Transfer ITS operatorship
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  its transfer-operatorship \
  --sender $CURRENT_ITS_OPERATOR \
  --to $RATE_LIMITER_EOA \
  send

# Verify the transfer by checking ITS config
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  query its config
```

### Step 4: Transfer Governance Operator to Relayer Operators EOA

**New Operator**: Relayer Operators EOA

| Network              | Target Address |
| -------------------- | -------------- |
| **Devnet Amplifier** | TBD            |

```bash
RELAYER_OPERATORS_EOA=<RELAYER_OPERATORS_EOA_ADDRESS>

# Transfer Governance operatorship
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  governance transfer-operatorship \
  --to $RELAYER_OPERATORS_EOA \
  send

# Verify the transfer by checking Governance config
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  query governance config
```

### Step 5: Update UpgradeAuthority for All Programs to Relayer Operators EOA

**Important**: This step requires using the Solana CLI to transfer program upgrade authority. All programs currently have the same upgrade authority.

**New UpgradeAuthority**: Relayer Operators EOA

| Network              | Target Address |
| -------------------- | -------------- |
| **Devnet Amplifier** | TBD            |

```bash
RELAYER_OPERATORS_EOA=<RELAYER_OPERATORS_EOA_ADDRESS>
CURRENT_UPGRADE_AUTHORITY=<CURRENT_UPGRADE_AUTHORITY_ADDRESS>

# Gateway
GATEWAY_PROGRAM_ID=$(jq -r '.chains["solana-devnet"].contracts.Gateway.address' ./axelar-chains-config/info/devnet-amplifier.json)
solana program set-upgrade-authority \
  $GATEWAY_PROGRAM_ID \
  --upgrade-authority $CURRENT_UPGRADE_AUTHORITY \
  --new-upgrade-authority $RELAYER_OPERATORS_EOA

# ITS (Interchain Token Service)
ITS_PROGRAM_ID=$(jq -r '.chains["solana-devnet"].contracts.InterchainTokenService.address' ./axelar-chains-config/info/devnet-amplifier.json)
solana program set-upgrade-authority \
  $ITS_PROGRAM_ID \
  --upgrade-authority $CURRENT_UPGRADE_AUTHORITY \
  --new-upgrade-authority $RELAYER_OPERATORS_EOA

# Governance
GOVERNANCE_PROGRAM_ID=$(jq -r '.chains["solana-devnet"].contracts.Governance.address' ./axelar-chains-config/info/devnet-amplifier.json)
solana program set-upgrade-authority \
  $GOVERNANCE_PROGRAM_ID \
  --upgrade-authority $CURRENT_UPGRADE_AUTHORITY \
  --new-upgrade-authority $RELAYER_OPERATORS_EOA

# Operators
OPERATORS_PROGRAM_ID=$(jq -r '.chains["solana-devnet"].contracts.Operators.address' ./axelar-chains-config/info/devnet-amplifier.json)
solana program set-upgrade-authority \
  $OPERATORS_PROGRAM_ID \
  --upgrade-authority $CURRENT_UPGRADE_AUTHORITY \
  --new-upgrade-authority $RELAYER_OPERATORS_EOA

# Gas Service
GAS_SERVICE_PROGRAM_ID=$(jq -r '.chains["solana-devnet"].contracts.GasService.address' ./axelar-chains-config/info/devnet-amplifier.json)
solana program set-upgrade-authority \
  $GAS_SERVICE_PROGRAM_ID \
  --upgrade-authority $CURRENT_UPGRADE_AUTHORITY \
  --new-upgrade-authority $RELAYER_OPERATORS_EOA

# Memo
MEMO_PROGRAM_ID=$(jq -r '.chains["solana-devnet"].contracts.Memo.address' ./axelar-chains-config/info/devnet-amplifier.json)
solana program set-upgrade-authority \
  $MEMO_PROGRAM_ID \
  --upgrade-authority $CURRENT_UPGRADE_AUTHORITY \
  --new-upgrade-authority $RELAYER_OPERATORS_EOA

# Verify all upgrade authorities
echo "Verifying Gateway upgrade authority:"
solana program show $GATEWAY_PROGRAM_ID | grep "Upgrade Authority"

echo "Verifying ITS upgrade authority:"
solana program show $ITS_PROGRAM_ID | grep "Upgrade Authority"

echo "Verifying Governance upgrade authority:"
solana program show $GOVERNANCE_PROGRAM_ID | grep "Upgrade Authority"

echo "Verifying Operators upgrade authority:"
solana program show $OPERATORS_PROGRAM_ID | grep "Upgrade Authority"

echo "Verifying Gas Service upgrade authority:"
solana program show $GAS_SERVICE_PROGRAM_ID | grep "Upgrade Authority"

echo "Verifying Memo upgrade authority:"
solana program show $MEMO_PROGRAM_ID | grep "Upgrade Authority"
```

### Step 6: Verify Gas Service Operator (PDA - No Action Required)

The Gas Service Operator is a Program Derived Address (PDA) controlled by the Operators Program. This is by design and does not require transfer. Verify the configuration:

```bash
# Verify Gas Service configuration
cargo run --release --bin solana-axelar-cli -- \
  --chain solana-devnet \
  query gas-service config
```

The operator should be a PDA derived from the Operators Program.

## Verification Checklist

After completing role transfers, verify all changes:

- [ ] Gateway `operator` is held by Emergency Operator EOA
- [ ] Gateway `UpgradeAuthority` is held by Relayer Operators EOA
- [ ] ITS `operator` is held by Rate Limiter EOA
- [ ] ITS `UpgradeAuthority` is held by Relayer Operators EOA
- [ ] Governance `operator` is held by Relayer Operators EOA
- [ ] Governance `UpgradeAuthority` is held by Relayer Operators EOA
- [ ] Operators `UpgradeAuthority` is held by Relayer Operators EOA
- [ ] Gas Service `UpgradeAuthority` is held by Relayer Operators EOA
- [ ] Gas Service `operator` (PDA) is controlled by Operators Program
- [ ] Memo `UpgradeAuthority` is held by Relayer Operators EOA
- [ ] All program upgrade authorities verified via `solana program show <PROGRAM_ID>`
- [ ] All operator configs verified via query commands
- [ ] Configuration updated in `devnet-amplifier.json` if necessary
- [ ] Documentation updated with new role addresses

## Notes

1. **Solana Program Model**: Solana uses a different permission model compared to EVM chains. Programs have an `UpgradeAuthority` that can upgrade the program code, while contracts within programs have `operator` or `owner` roles for operational tasks.

2. **Temporary UpgradeAuthority Assignment**: All UpgradeAuthority roles are **temporarily** assigned to Relayer Operators EOA because the Governance contract is not yet production-stable. **Once the Governance contract is stabilized**, these UpgradeAuthority roles should be transferred to the Governance program for proper decentralized governance.

3. **Program Derived Addresses (PDAs)**: The Gas Service operator is a PDA controlled by the Operators Program. PDAs are deterministic addresses derived from program IDs and seeds, and they cannot be transferred in the traditional sense.

4. **Token Manager Roles**: TokenManager Operator, Minter, and Flow Limiter roles are specific to individual token projects. Axelar does not manage these roles - they are owned and operated by the respective token project teams.

5. **Emergency Response**: The Emergency Operator EOA for Gateway is designated for rapid response scenarios where compromised signers need to be rotated quickly using the `rotate_signers` operation.

6. **ITS Pause Status**: The `set_pause_status` operation in ITS requires rapid response capability, which is why the ITS operator is assigned to the Rate Limiter EOA rather than going through governance.

7. **Operators Owner**: The Operators program owner is configured at initialization and should be set to the Relayer Operators EOA for operational registry management (adding/removing operators).

8. **Future Governance Migration**: When the Governance contract becomes stable, a follow-up release document should be created to transfer all UpgradeAuthority roles from Relayer Operators EOA to the Governance program.

9. **Solana CLI Requirements**: Ensure you have the Solana CLI installed and configured with the appropriate RPC endpoint and fee payer for executing these operations.

10. **Multi-Signature Requirements**: Depending on your operational security model, some of these operations may require multi-signature approval. Coordinate with the appropriate signers before executing transfers.

