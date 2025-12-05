## Consensus Chains Role Transfers & AxelarServiceGovernance Alignment v1.0.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                        |

| **Environment**      | **Chain**        | **Deployment Status** | **Date** |
| -------------------- | ---------------- | --------------------- | -------- |
| **Devnet Amplifier** | `core-ethereum`  | -                     | TBD      |
|                      | `core-avalanche` | -                     | TBD      |
|                      | `core-fantom`    | -                     | TBD      |
|                      | `core-moonbeam`  | -                     | TBD      |
|                      | `binance`        | -                     | TBD      |
|                      | `kava`           | -                     | TBD      |
| **Stagenet**         | `avalanche`      | -                     | TBD      |
|                      | `fantom`         | -                     | TBD      |
|                      | `moonbeam`       | -                     | TBD      |
|                      | `celo`           | -                     | TBD      |
|                      | `ethereum`       | -                     | TBD      |
| **Testnet**          | `avalanche`      | -                     | TBD      |
|                      | `fantom`         | -                     | TBD      |
|                      | `moonbeam`       | -                     | TBD      |
|                      | `celo`           | -                     | TBD      |
|                      | `ethereum`       | -                     | TBD      |
| **Mainnet**          | `avalanche`      | -                     | TBD      |
|                      | `fantom`         | -                     | TBD      |
|                      | `moonbeam`       | -                     | TBD      |
|                      | `celo`           | -                     | TBD      |
|                      | `ethereum`       | -                     | TBD      |

## Background

Rotate non‑critical roles to appropriate operational addresses, and assign critical roles to AxelarServiceGovernance. This enforces correct permissions, separation of duties, and stronger security.

### Role Transfer Summary

| Contract                | Role        | Current Role Owner             | Operations                                                                                         | Assign To                | Reasoning                                                                                |
| ----------------------- | ----------- | ------------------------------ | -------------------------------------------------------------------------------------------------- | ------------------------ | ---------------------------------------------------------------------------------------- |
| AxelarGateway           | deployer    | EOA                            | -                                                                                                  | -                        | Informational only (initial deployer EOA, no on-chain role to transfer)                  |
| AxelarGateway           | governance  | EOA (per-chain, see config)    | `transferGovernance`, `transferMintLimiter`, `upgrade`, `setTokenMintLimits`                       | AxelarServiceGovernance  | Critical protocol control over gateway upgrades, governance and mint limiter assignments |
| AxelarGateway           | mintLimiter | Multisig / EOA (see config)    | `setTokenMintLimits`, `transferMintLimiter`                                                        | Rate Limiter EOA         | Centralized per-token mint limit management with off-chain monitoring and response       |
| AxelarServiceGovernance | deployer    | EOA (per-environment deployer) | -                                                                                                  | -                        | Informational only (deployment provenance)                                               |
| AxelarGasService        | owner       | EOA (per-chain, see config)    | `upgrade`                                                                                          | AxelarServiceGovernance  | Critical protocol upgrade control over gas service implementation                        |
| AxelarGasService        | collector   | EOA (per-chain, see config)    | `collectFees`, `updateGasInfo`, `refund`                                                           | Operators                | Treasury and operational management of gas fee collection and refunds                    |
| Operators               | owner       | EOA                            | `addOperator`, `removeOperator`, `transferOwnership`, `proposeOwnership`                           | Relayer Operators EOA    | Operational registry management for relayer operators                                    |
| Operators               | operators   | EOAs (see config)              | `executeContract`                                                                                  | Relayer broadcaster EOAs | Frequent operational execution of contracts with operator privileges                     |
| InterchainTokenService  | deployer    | EOA                            | -                                                                                                  | -                        | Informational only (initial deployer EOA, no on-chain role to transfer)                  |
| InterchainTokenService  | owner       | EOA (per-chain, see config)    | `setTrustedAddress`, `removeTrustedAddress`, `setPauseStatus`, `migrateInterchainToken`, `upgrade` | AxelarServiceGovernance  | Operational token service management and upgrade control                                 |
| InterchainTokenService  | operator    | EOA                            | `setFlowLimits`, `transferOperatorship`, `proposeOperatorship`                                     | Rate Limiter EOA         | Operational flow limit management for cross-chain token flows                            |

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

### Step 1: Deploy AxelarServiceGovernance Contract

AxelarServiceGovernance should already be deployed on the governance chain and visible in the EVM config JSONs. If not, follow the steps below.

Key checks:

- `AxelarServiceGovernance` is deployed and verified on the governance chain.
- Each consensus chain has the correct `governanceChain`, `governanceAddress`, and `minimumTimeDelay` configured off-chain.

#### Configuration (if not deployed)

| Network              | `governanceAddress`                             | `minimumTimeDelay` | `deployer`                                   | `salt`                    | `operatorAddress`                            |
| -------------------- | ----------------------------------------------- | ------------------ | -------------------------------------------- | ------------------------- | -------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `0`                | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `v1.0.0 devnet-amplifier` | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `300`              | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `v1.0.0`                  | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `3600`             | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | `v1.0.0`                  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |
| **Mainnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `86400`            | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `v1.0.0`                  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

#### Add AxelarServiceGovernance config to `${ENV}.json`

For each consensus chain where AxelarServiceGovernance is not deployed, add the following configuration:

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

**Note**: The `deploy-contract.js` script supports parallel deployment using the `--parallel` flag.

**For a single chain:**

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance
```

**For all consensus chains in parallel:**

```bash
# Deploy to all consensus chains in parallel
ts-node evm/deploy-contract.js -c AxelarServiceGovernance --parallel
```

#### Verify Deployment

```bash
# Query the deployed address / owner
ts-node evm/governance.js --contractName AxelarServiceGovernance --action owner
```

```bash
# Verify AxelarServiceGovernance constructor / implementation via explorer
ts-node evm/verify-contract.js -c AxelarServiceGovernance --dir /path/to/axelar-gmp-sdk-solidity
```

### Step 2: Transfer AxelarGateway Governance Role

| Network              | Current Governance                                                                         | Target Address |
| -------------------- | ------------------------------------------------------------------------------------------ | -------------- |
| **Devnet-Amplifier** | `0xfB71a4d90c37C9BCE7cD4Cb692cE26EA3AC0A319`, `0x677c130e0f17F91F7361AcC15b8d7e6A3D6ECeeb` | TBD            |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                               | TBD            |
| **Testnet**          | `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`                                               | TBD            |
| **Mainnet**          | `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`                                               | TBD            |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')

- Note: Specify all the chain names on -n flag where we want proposal to run 

# Transfer governance to AxelarServiceGovernance
ts-node evm/governance.js schedule transferGovernance $ETA \
  --contractName InterchainGovernance \
  --targetContractName AxelarGateway \
  --newGovernance "$AXELAR_SERVICE_GOVERNANCE" 

# After ETA/minimumTimeDelay has passed, execute the proposal
ts-node evm/governance.js execute transferGovernance \
  --contractName InterchainGovernance \
  --targetContractName AxelarGateway \
  --newGovernance "$AXELAR_SERVICE_GOVERNANCE" --parallel

# Verify governance is now AxelarServiceGovernance
ts-node evm/gateway.js -e $ENV -n $CHAIN --action governance

# Verify transfer completed successfully
ts-node evm/governance.js --contractName AxelarGateway --action governance --parallel
```

### Step 3: Align AxelarGateway MintLimiter to Rate Limiter EOA

New mintLimiter: Rate Limiter EOA

| Network              | Current MintLimiter                                                                        | Target Address                               |
| -------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------- |
| **Devnet-Amplifier** | `0x3EE3DeA54E32B234Fd681509A19155978d1a3D18`, `0xD684531104B38326f41f144b7e710C1707E240F2` | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                               | `0xc81184546e7432b98e33a7184ea2423710344e7c` |
| **Testnet**          | `0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68`                                               | TBD                                          |
| **Mainnet**          | `0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68`                                               | TBD                                          |

```bash
# Get Rate Limiter EOA for current environment
MINT_LIMITER="<RATE_LIMITER_EOA_OR_MULTISIG>"

# Verify current mintLimiter
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action mintLimiter

# Transfer mintLimiter role
ts-node evm/governance.js --contractName AxelarGateway --action transferMintLimiter --newMintLimiter $MINT_LIMITER --parallel

# Verify transfer completed successfully
ts-node evm/governance.js --contractName AxelarGateway --action mintLimiter --parallel
```

### Step 4: Transfer AxelarGasService Owner Role

New owner: AxelarServiceGovernance.

| **Network**          | Current Owner                                | Target Address |
| -------------------- | -------------------------------------------- | -------------- |
| **Devnet-Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD            |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TBD            |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD            |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD            |

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

New owner: Operators Owner EOA

| **Network**          | Current Owner                                | Target Address                               |
| -------------------- | -------------------------------------------- | -------------------------------------------- |
| **Devnet-Amplifier** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | `0xd86fb81139f3bc86559ab495094fe2aa24b0a8af` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | Relayer Operators EOA                        |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | Relayer Operators EOA                        |

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

New owner: AxelarServiceGovernance.

| Network              | Current Owner                                | Target Address |
| -------------------- | -------------------------------------------- | -------------- |
| **Devnet-Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD            |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TBD            |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD            |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD            |

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

New operator: Rate Limiter EOA.

| Network              | Current Operator                             | Target Address                               |
| -------------------- | -------------------------------------------- | -------------------------------------------- |
| **Devnet-Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `0xc81184546e7432b98e33a7184ea2423710344e7c` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                                          |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                                          |

```bash
# Get Rate Limiter EOA for this environment
RATE_LIMITER_EOA="<RATE_LIMITER_EOA_ADDRESS>"

# Verify current operator (check if address is operator)
ts-node evm/its.js isOperator $RATE_LIMITER_EOA --parallel

# Transfer operatorship
ts-node evm/its.js transferOperatorship $RATE_LIMITER_EOA --parallel

# Verify transfer completed successfully
ts-node evm/its.js isOperator $RATE_LIMITER_EOA --parallel
```

## Verification Checklist

After completing role transfers for each consensus chain, verify:

- [ ] AxelarServiceGovernance is deployed and configured correctly.
- [ ] AxelarGateway governance is transferred to AxelarServiceGovernance.
- [ ] AxelarGateway mintLimiter is updated to the Rate Limiter EOA / mint-limiter multisig.
- [ ] AxelarGasService owner is transferred to AxelarServiceGovernance.
- [ ] Operators owner is transferred to the Relayer Operators EOA.
- [ ] InterchainTokenService owner is transferred to AxelarServiceGovernance.
- [ ] InterchainTokenService operator is transferred to the Rate Limiter EOA.
- [ ] All role transfers are verified on-chain.
- [ ] Contract addresses and roles are updated in `${ENV}.json`.
- [ ] Documentation is updated with new role addresses and reasoning.
