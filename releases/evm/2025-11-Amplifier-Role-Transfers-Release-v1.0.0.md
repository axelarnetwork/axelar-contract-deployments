# Amplifier Chains Role Transfers & AxelarServiceGovernance Alignment v1.0.0

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                       |

| **Environment** | **Chain** | **Deployment Status** | **Date** |
| --------------- | --------- | --------------------- | -------- |
| **Devnet Amplifier** | `avalanche-fuji` | - | TBD |
| | `berachain` | - | TBD |
| | `eth-sepolia` | - | TBD |
| | `flow` | - | TBD |
| | `optimism-sepolia` | - | TBD |
| | `plume-2` | - | TBD |
| | `xrpl-evm-devnet` | - | TBD |
| **Stagenet** | `berachain` | - | TBD |
| | `celo-sepolia` | - | TBD |
| | `flow` | - | TBD |
| | `hedera` | - | TBD |
| | `hyperliquid` | - | TBD |
| | `monad` | - | TBD |
| | `plume` | - | TBD |
| | `xrpl-evm` | - | TBD |
| **Testnet** | `berachain` | - | TBD |
| | `celo-sepolia` | - | TBD |
| | `flow` | - | TBD |
| | `hedera` | - | TBD |
| | `hyperliquid` | - | TBD |
| | `memento-demo` | - | TBD |
| | `monad` | - | TBD |
| | `plume` | - | TBD |
| | `xrpl-evm` | - | TBD |
| **Mainnet** | `berachain` | - | TBD |
| | `flow` | - | TBD |
| | `hedera` | - | TBD |
| | `hyperliquid` | - | TBD |
| | `monad` | - | TBD |
| | `plume` | - | TBD |
| | `xrpl-evm` | - | TBD |

## Background

Rotate non‑critical roles to appropriate operational addresses, and assign critical roles to AxelarServiceGovernance. This enforces correct permissions, separation of duties, and stronger security.

### Role Transfer Summary

| Contract | Role | Current Role Owner | Operations | Assign To | Reasoning |
|----------|------|----------------------|-----------|-----------|-----------|
| AxelarAmplifierGateway | owner | EOA | upgrade, transferOwnership, proposeOwnership, transferOperatorship | AxelarServiceGovernance | Critical protocol control over amplifier gateway upgrades and ownership/operatorship management |
| AxelarAmplifierGateway | operator | EOA | rotateSigners, transferOperatorship | Emergency Operator EOA | Emergency account to rotate to a prior honest verifier set if latest set is compromised |
| AxelarGasService | owner | EOA | upgrade | AxelarServiceGovernance | Critical protocol upgrade control over gas service implementation |
| Operators | owner | EOA | addOperator, removeOperator, transferOwnership, proposeOwnership | Relayer Operators EOA | Operational registry management for relayer operators |
| InterchainTokenService | owner | EOA | setTrustedAddress, removeTrustedAddress, setPauseStatus, migrateInterchainToken, upgrade | AxelarServiceGovernance | Operational token service management and upgrade control |
| InterchainTokenService | operator | EOA | setFlowLimits, transferOperatorship, proposeOperatorship | Rate Limiter EOA | Operational flow limit management for cross-chain token flows |

## Pre-requisites

1. Update npm dependencies (including contracts)
   ```bash
   npm ci && npm run build
   ```
2. Create an `.env` config
   ```yaml
   PRIVATE_KEY=<deployer private key>
   ENV=<devnet-amplifier|stagenet|testnet|mainnet>
   CHAIN=<chain name>
   ```
3. Verify current contract addresses and roles in `${ENV}.json` for each chain
4. TODO: Confirm `AxelarServiceGovernance` contract addresses for each environment
5. TODO: Confirm Emergency Operator EOA addresses for each environment
6. TODO: Confirm Relayer Operators EOA addresses for each environment
7. TODO: Confirm Rate Limiter EOA addresses for each environment
8. TODO: Confirm `Operators` contract addresses for each environment

## Deployment Steps

### Step 1: Deploy AxelarServiceGovernance (if not deployed)

**Note**: AxelarServiceGovernance contracts are not deployed on amplifier chains. They need to be deployed before role transfers can be executed via governance.

#### Configuration

| Network              | `governanceChain` | `governanceAddress`                              | `minimumTimeDelay` | `deployer`                                   |
| -------------------- | ----------------- | ------------------------------------------------ | ------------------ | -------------------------------------------- |
| **Devnet-amplifier** | `axelar`          | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`  | `0`                | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `axelar`          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `300`              | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `axelar`          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `3600`             | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `axelar`          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `86400`            | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

#### Add AxelarServiceGovernance config to `${ENV}.json`

For each amplifier chain, add the following configuration:

```json
{
  "AxelarServiceGovernance": {
    "governanceChain": "[governanceChain]",
    "governanceAddress": "[governanceAddress]",
    "minimumTimeDelay": [minimumTimeDelay],
    "deploymentMethod": "create2",
    "salt": "AxelarServiceGovernance [ENV]"
  }
}
```

#### Deploy AxelarServiceGovernance

**TODO**: 
- Confirm before deploying about using `create2` or `create3` method
- Confirm salt for AxelarServiceGovernance as well

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance -m create2/create3 -s "salt"
```

#### Verify Deployment

```bash
# Query the deployed address / owner
ts-node evm/governance.js -n $CHAIN --contractName AxelarServiceGovernance --action owner
```

```bash
# Verify AxelarServiceGovernance constructor / implementation via explorer
ts-node evm/verify-contract.js -e $ENV -n $CHAIN -c AxelarServiceGovernance --dir /path/to/axelar-gmp-sdk-solidity
```

### Step 2: Transfer AxelarAmplifierGateway Owner Role

**New Owner**: AxelarServiceGovernance contract

| Network              | Current Owner | Target Address                                    |
| -------------------- | ------------- | ------------------------------------------------- |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | AxelarServiceGovernance (governance contract address) |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | AxelarServiceGovernance (governance contract address) |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | AxelarServiceGovernance (governance contract address) |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | AxelarServiceGovernance (governance contract address) |

```bash
# Verify current owner
ts-node evm/ownership.js -c AxelarAmplifierGateway --action owner

# Transfer ownership to AxelarServiceGovernance
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')
ts-node evm/ownership.js -c AxelarAmplifierGateway --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```


### Step 3: Transfer AxelarAmplifierGateway Operator Role

**New Operator**: Emergency Operator EOA

| Network              | Current Operator | Target Address           |
| -------------------- | ---------------- | ------------------------ |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TODO: Emergency Operator EOA      |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TODO: Emergency Operator EOA      |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TODO: Emergency Operator EOA      |
| **Mainnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TODO: Emergency Operator EOA      |

```bash
# Get the Emergency Operator EOA address from the table above
EMERGENCY_OPERATOR_EOA="<EMERGENCY_OPERATOR_EOA_ADDRESS>"

# Verify current operator
# TODO: Add script to query operator role or use gateway.js

# Transfer operatorship
ts-node evm/gateway.js -n $CHAIN --action transferOperatorship --newOperator $EMERGENCY_OPERATOR_EOA
```

**TODO**: 
- Add verification step after transfer

### Step 4: Transfer AxelarGasService Owner Role

**New Owner**: AxelarServiceGovernance

| Network              | Current Owner | Target Address      |
| -------------------- | ------------- | ------------------- |
| **Devnet Amplifier** | Not set in config | TODO: AxelarServiceGovernance contract      |
| **Stagenet**         | Not set in config | TODO: AxelarServiceGovernance contract      |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TODO: AxelarServiceGovernance contract      |
| **Mainnet**          | Not set in config | TODO: AxelarServiceGovernance contract      |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE="<AXELAR_SERVICE_GOVERNANCE_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c AxelarGasService --action owner

# Transfer ownership to Multisig
ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

**TODO**: 
- Verify the current owner address for each chain before executing transfer
- Add verification step after transfer

### Step 5: Transfer Operators Owner Role

**New Owner**: Relayer Operators EOA

| Network              | Current Owner | Target Address      |
| -------------------- | ------------- | ------------------- |
| **Devnet Amplifier** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | TODO: Relayer Operators EOA      |
| **Stagenet**         | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | TODO: Relayer Operators EOA      |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`,`0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TODO: Relayer Operators EOA      |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TODO: Relayer Operators EOA      |

```bash
# Get the Relayer Operators EOA address from the table above
RELAYER_OPERATORS_EOA="<RELAYER_OPERATORS_EOA_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c Operators --action owner

# Transfer ownership to Multisig
ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $RELAYER_OPERATORS_EOA
```

### Step 6: Transfer InterchainTokenService Owner Role

**New Owner**: AxelarServiceGovernance

| Network              | Current Owner | Target Address      |
| -------------------- | ------------- | ------------------- |
| Devnet Amplifier     | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | AxelarServiceGovernance contract address      |
| Stagenet             | `0xBeF25f4733b9d451072416360609e5A4c115293E` | AxelarServiceGovernance contract address      |
| Testnet              | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | AxelarServiceGovernance contract address      |
| Mainnet              | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | AxelarServiceGovernance contract address      |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE="<AXELAR_SERVICE_GOVERNANCE_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c InterchainTokenService --action owner

# Transfer ownership to Multisig
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

**TODO**: 
- Verify the current owner address for each chain before executing transfer
- Add verification step after transfer

### Step 7: Transfer InterchainTokenService Operator Role

**New Operator**: Rate Limiter EOA

| Network              | Current Operator | Target Address      |
| -------------------- | ---------------- | ------------------- |
| **Devnet Amplifier** | Not set in config | TODO: Rate Limiter EOA      |
| **Stagenet**         | Not set in config | TODO: Rate Limiter EOA      |
| **Testnet**          | Not set in config | TODO: Rate Limiter EOA      |
| **Mainnet**          | Not set in config | TODO: Rate Limiter EOA      |

```bash
# Get the Rate Limiter EOA address from the table above
RATE_LIMITER_EOA="<RATE_LIMITER_EOA_ADDRESS>"

# Transfer operatorship
ts-node evm/its.js -n $CHAIN --action transferOperatorship --newOperator $RATE_LIMITER_EOA
```

## Verification Checklist

After completing role transfers for each chain, verify:

- [ ] AxelarServiceGovernance is deployed and configured correctly
- [ ] AxelarAmplifierGateway owner is transferred to AxelarServiceGovernance
- [ ] AxelarAmplifierGateway operator is transferred to Emergency Operator EOA
- [ ] AxelarGasService owner is transferred to AxelarServiceGovernance
- [ ] Operators owner is transferred to Relayer Operators EOA
- [ ] InterchainTokenService owner is transferred to AxelarServiceGovernance
- [ ] InterchainTokenService operator is transferred to Rate Limiter EOA
- [ ] All role transfers are verified on-chain
- [ ] Contract addresses are updated in `${ENV}.json`
- [ ] Documentation is updated with new role addresses
