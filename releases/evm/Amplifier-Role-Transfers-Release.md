# Amplifier Chains Role Transfers & InterchainGovernance Deployment

|                | **Owner**                             |
| -------------- | ------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io |
| **Deployment** |                                       |

| **Environment** | **Chain** | **Deployment Status** | **Date** |
| --------------- | --------- | --------------------- | -------- |
| **Devnet Amplifier** | `avalanche-fuji` | - | TBD |
| | `berachain` | - | TBD |
| | `eth-sepolia` | - | TBD |
| | `flow` | - | TBD |
| | `optimism-sepolia` | - | TBD |
| | `plume-2` | - | TBD |
| | `solana-12` | - | TBD |
| | `solana-2` | - | TBD |
| | `sui-2` | - | TBD |
| | `xrpl-dev` | - | TBD |
| | `xrpl-evm-devnet` | - | TBD |
| **Stagenet** | `berachain` | - | TBD |
| | `celo-sepolia` | - | TBD |
| | `flow` | - | TBD |
| | `hedera` | - | TBD |
| | `hyperliquid` | - | TBD |
| | `monad` | - | TBD |
| | `plume` | - | TBD |
| | `stellar-2025-q3` | - | TBD |
| | `sui` | - | TBD |
| | `xrpl` | - | TBD |
| | `xrpl-evm` | - | TBD |
| **Testnet** | `berachain` | - | TBD |
| | `celo-sepolia` | - | TBD |
| | `flow` | - | TBD |
| | `hedera` | - | TBD |
| | `hyperliquid` | - | TBD |
| | `memento-demo` | - | TBD |
| | `monad` | - | TBD |
| | `plume` | - | TBD |
| | `stellar-2025-q3` | - | TBD |
| | `sui` | - | TBD |
| | `xrpl` | - | TBD |
| | `xrpl-evm` | - | TBD |
| **Mainnet** | `berachain` | - | TBD |
| | `flow` | - | TBD |
| | `hedera` | - | TBD |
| | `hyperliquid` | - | TBD |
| | `monad` | - | TBD |
| | `plume` | - | TBD |
| | `stellar` | - | TBD |
| | `sui` | - | TBD |
| | `xrpl` | - | TBD |
| | `xrpl-evm` | - | TBD |

## Background

This release implements role transfers for critical protocol contracts on amplifier chains and deploys InterchainGovernance contracts where they are missing. The role transfers are necessary to move control from EOA addresses to governance contracts and multisigs for better security and operational management.

### Role Transfer Summary

| Contract | Role | Assign To | Operations |
|----------|------|-----------|------------|
| **AxelarAmplifierGateway** | owner | Governance | `upgrade()`, `transferOwnership()`, `proposeOwnership()`, `transferOperatorship()` |
| **AxelarAmplifierGateway** | operator | AXELARONS EOA | `rotateSigners()`, `transferOperatorship()` |
| **AxelarGasService** | owner | Multisig | `upgrade()` |
| **AxelarGasService** | collector | AXELARONS EOA | `collectFees()`, `updateGasInfo()`, `refund()` |
| **Operators** | owner | Multisig | `addOperator()`, `removeOperator()`, `transferOwnership()`, `proposeOwnership()` |
| **InterchainTokenService** | owner | Multisig | `setTrustedAddress()`, `removeTrustedAddress()`, `setPauseStatus()`, `migrateInterchainToken()` |
| **InterchainTokenService** | operator | Multisig | `setFlowLimits()`, `transferOperatorship()`, `proposeOperatorship()` |

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
4. TODO: Confirm governance contract addresses for each environment
5. TODO: Confirm multisig addresses for each environment
6. TODO: Confirm AXELARONS EOA addresses for each environment

## Deployment Steps

### Step 1: Deploy InterchainGovernance (if not deployed)

**Note**: InterchainGovernance contracts are not deployed on amplifier chains. They need to be deployed before role transfers can be executed via governance.

#### Configuration

| Network              | `governanceChain` | `governanceAddress`                              | `minimumTimeDelay` | `deployer`                                   |
| -------------------- | ----------------- | ------------------------------------------------ | ------------------ | -------------------------------------------- |
| **Devnet-amplifier** | `axelar`          | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`  | `0`                | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` |
| **Stagenet**         | `axelar`          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `300`              | `0xBeF25f4733b9d451072416360609e5A4c115293E` |
| **Testnet**          | `axelar`          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `3600`             | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` |
| **Mainnet**          | `axelar`          | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`  | `86400`            | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` |

#### Add InterchainGovernance config to `${ENV}.json`

For each amplifier chain, add the following configuration:

```json
{
  "InterchainGovernance": {
    "governanceChain": "[governanceChain]",
    "governanceAddress": "[governanceAddress]",
    "minimumTimeDelay": [minimumTimeDelay],
    "deploymentMethod": "create2",
    "salt": "InterchainGovernance [ENV]"
  }
}
```

#### Deploy InterchainGovernance

```bash
ts-node evm/deploy-contract.js -c InterchainGovernance -m create2
```

#### Verify Deployment

```bash
# Query the deployed address
ts-node evm/governance.js -n $CHAIN --contractName InterchainGovernance --action owner

# Verify governance chain and address are set correctly
# TODO: Add verification script or manual check
```

### Step 2: Transfer AxelarAmplifierGateway Owner Role

**New Owner**: Governance Contract (InterchainGovernance)

| Network              | Current Owner | Target Address                                    |
| -------------------- | ------------- | ------------------------------------------------- |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | InterchainGovernance (deployed address)          |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | InterchainGovernance (deployed address)          |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | InterchainGovernance (deployed address)         |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | InterchainGovernance (deployed address)          |

```bash
# Verify current owner
ts-node evm/ownership.js -c AxelarAmplifierGateway --action owner

# Transfer ownership to InterchainGovernance
INTERCHAIN_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.InterchainGovernance.address" | tr -d '"')
ts-node evm/ownership.js -c AxelarAmplifierGateway --action transferOwnership --newOwner $INTERCHAIN_GOVERNANCE
```

**TODO**: 
- Add verification step after transfer

### Step 3: Transfer AxelarAmplifierGateway Operator Role

**New Operator**: AXELARONS EOA

| Network              | Current Operator | Target Address           |
| -------------------- | ---------------- | ------------------------ |
| **Devnet Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TODO: AXELARONS EOA      |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | TODO: AXELARONS EOA      |
| **Testnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TODO: AXELARONS EOA      |
| **Mainnet**          | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TODO: AXELARONS EOA      |

```bash
# Get the AXELARONS EOA address from the table above
AXELARONS_EOA="<AXELARONS_EOA_ADDRESS>"

# Verify current operator
# TODO: Add script to query operator role or use gateway.js

# Transfer operatorship
ts-node evm/gateway.js -n $CHAIN --action transferOperatorship --newOperator $AXELARONS_EOA
```

**TODO**: 
- Add verification step after transfer

### Step 4: Transfer AxelarGasService Owner Role

**New Owner**: Multisig

| Network              | Current Owner | Target Address      |
| -------------------- | ------------- | ------------------- |
| **Devnet Amplifier** | Not set in config | TODO: Multisig      |
| **Stagenet**         | Not set in config | TODO: Multisig      |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TODO: Multisig      |
| **Mainnet**          | Not set in config | TODO: Multisig      |

```bash
# Get the multisig address from the table above
MULTISIG_ADDRESS="<MULTISIG_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c AxelarGasService --action owner

# Transfer ownership to Multisig
ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner $MULTISIG_ADDRESS
```

**TODO**: 
- Verify the current owner address for each chain before executing transfer
- Add verification step after transfer

### Step 5: Transfer AxelarGasService Collector Role

**New Collector**: AXELARONS EOA

| Network              | Current Collector | Target Address           |
| -------------------- | ----------------- | ------------------------ |
| **Devnet Amplifier** | `0x505381EEd15c828b3158836d0196bC2E6B51c49f`,`0x2e1C331cE54863555Ee1638c99eA9154b02bA831`,`0x217D3F23884beD1B13177DaC309634E4A30fe5F1` | TODO: AXELARONS EOA      |
| **Stagenet**         | `0xc5C525B7Bb2a7Ce95C13Ee5aBdB7F8fd3cb77392` | TODO: AXELARONS EOA      |
| **Testnet**          | `0x7F83F5cA2AE4206AbFf8a3C3668e88ce5F11C0B5`,`0x7AC8A53528bD497d7Ac8AEC4CcfDbA556e32BDD6` | TODO: AXELARONS EOA      |
| **Mainnet**          | `0x7DdB2d76b80B0AA19bDEa48EB1301182F4CeefbC` | TODO: AXELARONS EOA      |

```bash
# Get the AXELARONS EOA address from the table above
AXELARONS_EOA="<AXELARONS_EOA_ADDRESS>"

# Verify current collector (query contract directly)
GAS_SERVICE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarGasService.address" | tr -d '"')

# Update collector via upgrade (collector is set in constructor, requires upgrade)
ts-node evm/deploy-upgradable.js -c AxelarGasService -m create2 --args "{\"collector\": \"$AXELARONS_EOA\"}" --reuseProxy
```

**TODO**: 
- Verify the current collector address for each chain before executing transfer
- Add verification step after transfer

### Step 6: Transfer Operators Owner Role

**New Owner**: Multisig

| Network              | Current Owner | Target Address      |
| -------------------- | ------------- | ------------------- |
| **Devnet Amplifier** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | TODO: Multisig      |
| **Stagenet**         | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | TODO: Multisig      |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05`,`0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | TODO: Multisig      |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TODO: Multisig      |

```bash
# Get the multisig address from the table above
MULTISIG_ADDRESS="<MULTISIG_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c Operators --action owner

# Transfer ownership to Multisig
ts-node evm/ownership.js -c Operators --action transferOwnership --newOwner $MULTISIG_ADDRESS
```

**TODO**: 
- Add verification step after transfer

### Step 7: Transfer InterchainTokenService Owner Role

**New Owner**: Multisig

| Network              | Current Owner | Target Address      |
| -------------------- | ------------- | ------------------- |
| **Devnet Amplifier** | Not set in config | TODO: Multisig      |
| **Stagenet**         | Not set in config | TODO: Multisig      |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TODO: Multisig      |
| **Mainnet**          | Not set in config | TODO: Multisig      |

```bash
# Get the multisig address from the table above
MULTISIG_ADDRESS="<MULTISIG_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c InterchainTokenService --action owner

# Transfer ownership to Multisig
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner $MULTISIG_ADDRESS
```

**TODO**: 
- Verify the current owner address for each chain before executing transfer
- Add verification step after transfer

### Step 8: Transfer InterchainTokenService Operator Role

**New Operator**: Multisig

| Network              | Current Operator | Target Address      |
| -------------------- | ---------------- | ------------------- |
| **Devnet Amplifier** | Not set in config | TODO: Multisig      |
| **Stagenet**         | Not set in config | TODO: Multisig      |
| **Testnet**          | Not set in config | TODO: Multisig      |
| **Mainnet**          | Not set in config | TODO: Multisig      |

```bash
# Get the multisig address from the table above
MULTISIG_ADDRESS="<MULTISIG_ADDRESS>"

# Transfer operatorship
ts-node evm/its.js -n $CHAIN --action transferOperatorship --newOperator $MULTISIG_ADDRESS
```

**TODO**: 
- Verify the correct command/script to transfer ITS operatorship
- Add verification step after transfer

## Verification Checklist

After completing role transfers for each chain, verify:

- [ ] InterchainGovernance is deployed and configured correctly
- [ ] AxelarAmplifierGateway owner is transferred to InterchainGovernance
- [ ] AxelarAmplifierGateway operator is transferred to AXELARONS EOA
- [ ] AxelarGasService owner is transferred to Multisig
- [ ] AxelarGasService collector is transferred to AXELARONS EOA
- [ ] Operators owner is transferred to Multisig
- [ ] InterchainTokenService owner is transferred to Multisig
- [ ] InterchainTokenService operator is transferred to Multisig
- [ ] All role transfers are verified on-chain
- [ ] Contract addresses are updated in `${ENV}.json`
- [ ] Documentation is updated with new role addresses
