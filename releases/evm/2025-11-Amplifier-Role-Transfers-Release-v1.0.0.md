# Amplifier Chains Role Transfers & AxelarServiceGovernance Alignment v1.0.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                        |

| **Environment**      | **Chain**          | **Deployment Status** | **Date** |
| -------------------- | ------------------ | --------------------- | -------- |
| **Devnet Amplifier** | `avalanche-fuji`   | -                     | TBD      |
|                      | `berachain`        | -                     | TBD      |
|                      | `eth-sepolia`      | -                     | TBD      |
|                      | `flow`             | -                     | TBD      |
|                      | `optimism-sepolia` | -                     | TBD      |
|                      | `plume-2`          | -                     | TBD      |
|                      | `xrpl-evm-devnet`  | -                     | TBD      |
| **Stagenet**         | `berachain`        | -                     | TBD      |
|                      | `celo-sepolia`     | -                     | TBD      |
|                      | `flow`             | -                     | TBD      |
|                      | `hedera`           | -                     | TBD      |
|                      | `hyperliquid`      | -                     | TBD      |
|                      | `monad`            | -                     | TBD      |
|                      | `plume`            | -                     | TBD      |
|                      | `xrpl-evm`         | -                     | TBD      |
| **Testnet**          | `berachain`        | -                     | TBD      |
|                      | `celo-sepolia`     | -                     | TBD      |
|                      | `flow`             | -                     | TBD      |
|                      | `hedera`           | -                     | TBD      |
|                      | `hyperliquid`      | -                     | TBD      |
|                      | `memento-demo`     | -                     | TBD      |
|                      | `monad`            | -                     | TBD      |
|                      | `plume`            | -                     | TBD      |
|                      | `xrpl-evm`         | -                     | TBD      |
| **Mainnet**          | `berachain`        | -                     | TBD      |
|                      | `flow`             | -                     | TBD      |
|                      | `hedera`           | -                     | TBD      |
|                      | `hyperliquid`      | -                     | TBD      |
|                      | `monad`            | -                     | TBD      |
|                      | `plume`            | -                     | TBD      |
|                      | `xrpl-evm`         | -                     | TBD      |

## Background

Rotate non‑critical roles to appropriate operational addresses, and assign critical roles to AxelarServiceGovernance. This enforces correct permissions, separation of duties, and stronger security.

### Role Transfer Summary

| Contract               | Role     | Current Role Owner | Operations                                                                               | Assign To               | Reasoning                                                                                       |
| ---------------------- | -------- | ------------------ | ---------------------------------------------------------------------------------------- | ----------------------- | ----------------------------------------------------------------------------------------------- |
| AxelarAmplifierGateway | owner    | EOA                | upgrade, transferOwnership, proposeOwnership, transferOperatorship                       | AxelarServiceGovernance | Critical protocol control over amplifier gateway upgrades and ownership/operatorship management |
| AxelarAmplifierGateway | operator | EOA                | rotateSigners, transferOperatorship                                                      | Emergency Operator EOA  | Emergency account to rotate to a prior honest verifier set if latest set is compromised         |
| AxelarGasService       | owner    | EOA                | upgrade                                                                                  | AxelarServiceGovernance | Critical protocol upgrade control over gas service implementation                               |
| Operators              | owner    | EOA                | addOperator, removeOperator, transferOwnership, proposeOwnership                         | Relayer Operators EOA   | Operational registry management for relayer operators                                           |
| InterchainTokenService | owner    | EOA                | setTrustedAddress, removeTrustedAddress, setPauseStatus, migrateInterchainToken, upgrade | AxelarServiceGovernance | Operational token service management and upgrade control                                        |
| InterchainTokenService | operator | EOA                | setFlowLimits, transferOperatorship, proposeOperatorship                                 | Rate Limiter EOA        | Operational flow limit management for cross-chain token flows                                   |

## Pre-requisites

1. Update npm dependencies
    ```bash
    npm ci && npm run build
    ```
2. Create an `.env` config
    ```yaml
    PRIVATE_KEY=<deployer private key>
    ENV=<devnet-amplifier|stagenet|testnet|mainnet>
    CHAIN=<chain name>
    ```

## Deployment Steps

### Step 1: Deploy AxelarServiceGovernance (if not deployed)

**Note**: AxelarServiceGovernance contracts are not deployed on amplifier chains. They need to be deployed before role transfers can be executed via governance.

#### Configuration

| Network              | `governanceAddress`                             | `minimumTimeDelay` | `deployer`                                   | `salt`                    | `operatorAddress`                            |
| -------------------- | ----------------------------------------------- | ------------------ | -------------------------------------------- | ------------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `0`                | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `v1.0.0 devnet-amplifier` | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `300`              | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `v1.0.0`                  | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `3600`             | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `v1.0.0`                  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `86400`            | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `v1.0.0`                  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

#### Add AxelarServiceGovernance config to `${ENV}.json`

For each amplifier chain, add the following configuration:

```json
{
  "AxelarServiceGovernance": {
    "governanceChain": "Axelarnet",
    "governanceAddress": "[governanceAddress]",
    "minimumTimeDelay": [minimumTimeDelay],
    "operator": "[operatorAddress]",
    "deploymentMethod": "create2",
    "salt": "[salt]"
  }
}
```

#### Deploy AxelarServiceGovernance

**Note**: The `deploy-contract.js` script supports parallel deployment using the `--parallel` flag. To deploy on all amplifier chains, first get the amplifier chain names using the `getAmplifierChains` helper function.

**For a single chain:**

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance
```

**For all amplifier chains in parallel:**

```bash
# Deploy to all amplifier chains in parallel
ts-node evm/deploy-contract.js -c AxelarServiceGovernance --parallel
```

#### Verify Deployment

```bash
# Query the deployed address / owner
ts-node evm/governance.js -n $CHAIN --contractName AxelarServiceGovernance --action owner
```

```bash
# Verify AxelarServiceGovernance constructor / implementation via explorer
ts-node evm/verify-contract.js -c AxelarServiceGovernance --dir /path/to/axelar-gmp-sdk-solidity
```

### Step 2: Transfer AxelarAmplifierGateway Owner Role

**New Owner**: AxelarServiceGovernance contract

| Network              | Current Owner                                | Axelar Service Governance |
| -------------------- | -------------------------------------------- | ------------------------- |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD                       |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TBD                       |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                       |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                       |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')

# Verify current owner
ts-node evm/ownership.js -c AxelarGateway --action owner --parallel

# Transfer ownership to AxelarServiceGovernance
ts-node evm/ownership.js -c AxelarGateway --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE  --parallel

# Verify transfer completed successfully
ts-node evm/ownership.js -c AxelarGateway --action owner --parallel
```

### Step 3: Transfer AxelarAmplifierGateway Operator Role

**New Operator**: Emergency Operator EOA

| Network              | Current Operator                             | Emergency Operator EOA                       |
| -------------------- | -------------------------------------------- | -------------------------------------------- |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `0x5cb17b091c0d9ed922862ff7843fb1f865f925c9` |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TBD                                          |
| **Mainnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TBD                                          |

```bash
# Get the Emergency Operator EOA address from the table above
EMERGENCY_OPERATOR_EOA="<EMERGENCY_OPERATOR_EOA_ADDRESS>"

# Transfer operatorship
ts-node evm/gateway.js -n $CHAIN --action transferOperatorship --newOperator $EMERGENCY_OPERATOR_EOA  --parallel

# Verify transfer completed successfully
ts-node evm/gateway.js -n $CHAIN --action operator --parallel
```

### Step 4: Transfer AxelarGasService Owner Role

**New Owner**: AxelarServiceGovernance

| Network              | Current Owner                                | Axelar Service Governance |
| -------------------- | -------------------------------------------- | ------------------------- |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD                       |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TBD                       |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                       |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                       |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')

# Verify current owner
ts-node evm/ownership.js -c AxelarGasService --action owner --parallel

# Transfer ownership to AxelarServiceGovernance
ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE --parallel

# Verify transfer completed successfully
ts-node evm/ownership.js -c AxelarGasService --action owner --parallel
```

### Step 5: Transfer Operators Owner Role

**New Owner**: Operators Owner EOA

| Network              | Current Owner                                                                              | Operators Owner EOA                    |
| -------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------- |
| **Devnet Amplifier** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df`                                               | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df`                                               | `0xd86fb81139f3bc86559ab495094fe2aa24b0a8af` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`, `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TBD                                          |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`                                               | TBD                                          |

```bash
# Get the Operators Owner EOA address from the table above
OPERATORS_OWNER_EOA="<OPERATORS_OWNER_EOA_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c Operators --action owner --parallel

# Transfer ownership to Operators Owner EOA
ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $OPERATORS_OWNER_EOA --parallel

# Verify transfer completed successfully
ts-node evm/ownership.js -c Operators --action owner --parallel
```

### Step 6: Transfer InterchainTokenService Owner Role

**New Owner**: AxelarServiceGovernance

| Network              | Current Owner                                | Axelar Service Governance |
| -------------------- | -------------------------------------------- | ------------------------- |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD                       |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TBD                       |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                       |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                       |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')

# Verify current owner
ts-node evm/ownership.js -c InterchainTokenService --action owner --parallel

# Transfer ownership to AxelarServiceGovernance
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE --parallel

# Verify transfer completed successfully
ts-node evm/ownership.js -c InterchainTokenService --action owner --parallel
```

### Step 7: Transfer InterchainTokenService Operator Role

**New Operator**: Rate Limiter EOA

| Network              | Current Operator  | Rate Limiter EOA                             |
| -------------------- | ----------------- | -------------------------------------------- |
| **Devnet Amplifier** | Not set in config | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | Not set in config | `0xc81184546e7432b98e33a7184ea2423710344e7c` |
| **Testnet**          | Not set in config | TBD                                          |
| **Mainnet**          | Not set in config | TBD                                          |

**Note**: If the operator is not set in the config, check the current operator on-chain before transferring. If the operator is `address(0)`, the owner must set the operator first before transferring.

```bash
# Get the Rate Limiter EOA address from the table above
RATE_LIMITER_EOA="<RATE_LIMITER_EOA_ADDRESS>"

# Verify current owner
ts-node evm/its.js operator --parallel

# Transfer operatorship
ts-node evm/its.js transferOperatorship $RATE_LIMITER_EOA --parallel

# Verify transfer completed successfully
ts-node evm/its.js operator --parallel
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
