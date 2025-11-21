## Consensus Chains Role Transfers & AxelarServiceGovernance Alignment v1.0.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                        |

| **Environment** | **Chain**        | **Deployment Status** | **Date** |
| --------------- | ---------------- | --------------------- | -------- |
| **Devnet Amplifier** | `ethereum-sepolia` | -                 | TBD      |
|                 | `avalanche`      | -                     | TBD      |
|                 | `fantom`         | -                     | TBD      |
|                 | `moonbeam`       | -                     | TBD      |
|                 | `binance`        | -                     | TBD      |
|                 | `kava`           | -                     | TBD      |
| **Stagenet**     | `avalanche`      | -                     | TBD      |
|                 | `fantom`         | -                     | TBD      |
|                 | `moonbeam`       | -                     | TBD      |
|                 | `celo`           | -                     | TBD      |
|                 | `ethereum`       | -                     | TBD      |
| **Testnet**      | `avalanche`      | -                     | TBD      |
|                 | `fantom`         | -                     | TBD      |
|                 | `moonbeam`       | -                     | TBD      |
|                 | `celo`           | -                     | TBD      |
|                 | `ethereum`       | -                     | TBD      |
| **Mainnet**      | `avalanche`      | -                     | TBD      |
|                 | `fantom`         | -                     | TBD      |
|                 | `moonbeam`       | -                     | TBD      |
|                 | `celo`           | -                     | TBD      |
|                 | `ethereum`       | -                     | TBD      |

## Background

Rotate non‑critical roles to appropriate operational addresses, and assign critical roles to AxelarServiceGovernance. This enforces correct permissions, separation of duties, and stronger security.

### Role Transfer Summary

| Contract               | Role        | Current Role Owner                               | Operations                                                                                         | Assign To                 | Reasoning                                                   |
|------------------------|------------|----------------------------------------------------|----------------------------------------------------------------------------------------------------|--------------------------|-------------------------------------------------------------|
| AxelarGateway          | deployer    | EOA                                               | -                                                                                                  | -                        | Informational only (initial deployer EOA, no on-chain role to transfer) |
| AxelarGateway          | governance  | EOA (per-chain, see config)                       | `transferGovernance`, `transferMintLimiter`, `upgrade`, `setTokenMintLimits`                      | AxelarServiceGovernance  | Critical protocol control over gateway upgrades, governance and mint limiter assignments |
| AxelarGateway          | mintLimiter | Multisig / EOA (see config)                       | `setTokenMintLimits`, `transferMintLimiter`                                                       | Rate Limiter EOA         | Centralized per-token mint limit management with off-chain monitoring and response       |
| AxelarServiceGovernance | deployer   | EOA (per-environment deployer)                    | -                                                                                                  | -                        | Informational only (deployment provenance)                  |
| AxelarGasService       | owner       | EOA (per-chain, see config)                       | `upgrade`                                                                                          | AxelarServiceGovernance  | Critical protocol upgrade control over gas service implementation          |
| AxelarGasService       | collector   | EOA (per-chain, see config)                       | `collectFees`, `updateGasInfo`, `refund`                                                          | Operators                | Treasury and operational management of gas fee collection and refunds     |
| Operators              | owner       | EOA                                               | `addOperator`, `removeOperator`, `transferOwnership`, `proposeOwnership`                          | Relayer Operators EOA    | Operational registry management for relayer operators       |
| Operators              | operators   | EOAs (see config)                                 | `executeContract`                                                                                  | Relayer broadcaster EOAs | Frequent operational execution of contracts with operator privileges       |
| InterchainTokenService | deployer    | EOA                                               | -                                                                                                  | -                        | Informational only (initial deployer EOA, no on-chain role to transfer) |
| InterchainTokenService | owner       | EOA (per-chain, see config)                       | `setTrustedAddress`, `removeTrustedAddress`, `setPauseStatus`, `migrateInterchainToken`, `upgrade` | AxelarServiceGovernance  | Operational token service management and upgrade control    |
| InterchainTokenService | operator    | EOA                                               | `setFlowLimits`, `transferOperatorship`, `proposeOperatorship`                                    | Rate Limiter EOA         | Operational flow limit management for cross-chain token flows |

## Pre-requisites

1. Update npm dependencies

```bash
npm ci && npm run build
```

2. Create an `.env` config

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet|stagenet|testnet|mainnet>
CHAIN=<chain name>
```

## Deployment Steps

### Step 1: Deploy AxelarServiceGovernance Contract

AxelarServiceGovernance should already be deployed on the governance chain and visible in the EVM config JSONs. If not, follow the steps below.

Key checks:

- `AxelarServiceGovernance` is deployed and verified on the governance chain.
- Each consensus chain has the correct `governanceChain`, `governanceAddress`, and `minimumTimeDelay` configured off-chain.


#### Configuration (if not deployed)

| Network              | `governanceAddress`                              | `minimumTimeDelay` | `deployer`                                   |
| -------------------- | ------------------------------------------------ | ------------------ | -------------------------------------------- |
| **Devnet-amplifier** | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`  | `0`                | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `300`              | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `3600`             | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `86400`            | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

#### Add AxelarServiceGovernance config to `${ENV}.json`

For each amplifier chain, add the following configuration:

```json
{
  "AxelarServiceGovernance": {
    "governanceChain": "axelar",
    "governanceAddress": "[governanceAddress]",
    "minimumTimeDelay": [minimumTimeDelay],
    "deploymentMethod": "TBD",
    "salt": "TBD"
  }
}
```

#### Deploy AxelarServiceGovernance

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance 
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

### Step 2: Transfer AxelarGateway Governance Role

| Network | Current Governance                    | Target Address                              |
| ------- | ------------------------------------- | ------------------------------------------ |
| **Devnet-Amplifier**   | Not set in config      | TBD     |
| **Stagenet** | Not set in config                | TBD     |
| **Testnet**  | `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`, `0x4F0f42bF41ba60895134EC99dE79A041E5269003` | TBD     |
| **Mainnet**  | `0xBbEE71e2fE7741Cdd7787DC46D73Af6715D47Dc0`                | TBD     |

```bash
# Verify current governance
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action governance


# Transfer governance to AxelarServiceGovernance
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action transferGovernance --newGovernance $AXELAR_SERVICE_GOVERNANCE
```

### Step 3: Align AxelarGateway MintLimiter to Rate Limiter EOA

New mintLimiter: Rate Limiter EOA / mint-limiter multisig.

| Network  | Current MintLimiter               | Target Address                    |
| -------- | --------------------------------- | --------------------------------- |
| **Devnet-Amplifier**   | Not set in config | TBD |
| **Stagenet** | Not set in config | TBD |
| **Testnet**  | Not set in config | TBD |
| **Mainnet**  | Not set in config | TBD |

```bash
# Get Rate Limiter EOA / mint-limiter multisig for this environment
MINT_LIMITER="<RATE_LIMITER_EOA_OR_MULTISIG>"

# Verify current mintLimiter
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action mintLimiter

# Transfer mintLimiter role
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action transferMintLimiter --newMintLimiter $MINT_LIMITER
```

### Step 4: Transfer AxelarGasService Owner Role

New owner: AxelarServiceGovernance.

| **Network**  | Current Owner                    | Target Address                       |
| ------------ | -------------------------------- | ------------------------------------ |
| **Devnet**   | Not set in config                | TBD |
| **Stagenet** | Not set in config                | TBD |
| **Testnet**  | Not set in config                | TBD |
| **Mainnet**  | Not set in config                | TBD |

```bash
# Get AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE="<AXELAR_SERVICE_GOVERNANCE_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c AxelarGasService --action owner

# Transfer ownership to AxelarServiceGovernance
ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

### Step 5: Transfer Operators Owner Role

New owner: Relayer Operators EOA.

| **Network**  | Current Owner                                 | Target Address          |
| -------- | --------------------------------------------- | ----------------------- |
| **Devnet-Amplifier**   | Not set in config      | Relayer Operators EOA   |
| **Stagenet** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df`  | Relayer Operators EOA   |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`  | Relayer Operators EOA   |
| **Mainnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`  | Relayer Operators EOA   |

```bash
# Get the Relayer Operators EOA for this environment
RELAYER_OPERATORS_EOA="<RELAYER_OPERATORS_EOA_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c Operators --action owner

# Transfer ownership
ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $RELAYER_OPERATORS_EOA
```

### Step 6: Transfer InterchainTokenService Owner Role

New owner: AxelarServiceGovernance.

| Network  | Current Owner               | Target Address                       |
| -------- | --------------------------- | ------------------------------------ |
| **Devnet-Amplifier**   | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TBD |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD |
| **Mainnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD |

```bash
# Get AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE="<AXELAR_SERVICE_GOVERNANCE_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c InterchainTokenService --action owner

# Transfer ownership
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

### Step 7: Transfer InterchainTokenService Operator Role

New operator: Rate Limiter EOA.

| Network  | Current Operator         | Target Address      |
| -------- | ------------------------- | ------------------- |
| **Devnet-Amplifier**  | Not set in config | TBD    |
| **Stagenet** | Not set in config | TBD    |
| **Testnet**  | Not set in config | TBD    |
| **Mainnet**  | Not set in config | TBD    |

```bash
# Get Rate Limiter EOA for this environment
RATE_LIMITER_EOA="<RATE_LIMITER_EOA_ADDRESS>"

# Transfer operatorship
ts-node evm/its.js -n $CHAIN --action transferOperatorship --newOperator $RATE_LIMITER_EOA
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
