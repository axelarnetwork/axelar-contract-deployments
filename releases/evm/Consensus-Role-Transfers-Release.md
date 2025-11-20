## Consensus Chains Role Transfers & AxelarServiceGovernance Alignment

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

This release implements role transfers for critical protocol contracts on consensus-connected EVM chains and aligns them with AxelarServiceGovernance, rate limiting, and operational EOAs. The goal is to move control from EOAs to governance contracts and designated multisigs where appropriate, while keeping high-frequency operational actions on specialized EOAs.

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

1. Update npm dependencies (including contracts)

```bash
npm ci && npm run build
```

2. Create an `.env` config

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet|stagenet|testnet|mainnet>
CHAIN=<chain name>
```

3. Verify current contract addresses and roles in `${ENV}.json` for each consensus chain (excluding Sui, Stellar, Solana).
4. Confirm AxelarServiceGovernance contract addresses for each environment.
5. Confirm Rate Limiter EOA addresses for each environment.
6. Confirm Relayer Operators EOA and broadcaster EOAs for each environment.
7. Confirm `Operators` contract addresses for each environment.

## Deployment Steps

### Step 1: Ensure AxelarServiceGovernance is Deployed

AxelarServiceGovernance should already be deployed on the governance chain and visible in the EVM config JSONs. If not, follow the governance deployment playbook and add it under `contracts.AxelarServiceGovernance` for each chain.

Key checks:

- `AxelarServiceGovernance` is deployed and verified on the governance chain.
- Each consensus chain has the correct `governanceChain`, `governanceAddress`, and `minimumTimeDelay` configured off-chain.

### Step 2: Transfer AxelarGateway Governance Role

New governance: AxelarServiceGovernance (governance contract).

| Network | Current Governance                                                                                 | Target Address                                    |
| -------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------- |
| **Devnet-Amplifier**   | Not set in config                                                                    | AxelarServiceGovernance (governance contract)     |
| **Stagenet** | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                                   | AxelarServiceGovernance (governance contract)     |
| **Testnet**  | `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`, `0x4F0f42bF41ba60895134EC99dE79A041E5269003` (per-chain) | AxelarServiceGovernance (governance contract)     |
| **Mainnet**  | `0xBbEE71e2fE7741Cdd7787DC46D73Af6715D47Dc0`                                                   | AxelarServiceGovernance (governance contract)     |

```bash
# Verify current governance
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action governance

# Load AxelarServiceGovernance address from config
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')

# Transfer governance to AxelarServiceGovernance
ts-node evm/governance.js -n $CHAIN --contractName AxelarGateway --action transferGovernance --newGovernance $AXELAR_SERVICE_GOVERNANCE
```

### Step 3: Align AxelarGateway MintLimiter to Rate Limiter EOA / Multisig

New mintLimiter: Rate Limiter EOA / mint-limiter multisig.

| Network  | Current MintLimiter                          | Target Address                    |
| -------- | -------------------------------------------- | --------------------------------- |
| **Devnet-Amplifier**   | Not set in config | Rate Limiter EOA / mint-limiter multisig |
| **Stagenet** | Not set in config | Rate Limiter EOA / mint-limiter multisig |
| **Testnet**  | `0xF0E17583C906f3e672e591791a88c1116F53081c`, `0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68` (per-chain) | Rate Limiter EOA / mint-limiter multisig |
| **Mainnet**  | `0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68` | Rate Limiter EOA / mint-limiter multisig |

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

| **Network**  | Current Owner                                                                         | Target Address                       |
| -------- | ------------------------------------------------------------------------------------- | ------------------------------------ |
| **Devnet**   | Not set in config                                                                    | AxelarServiceGovernance (governance) |
| **Stagenet** | Not set in config                                                                    | AxelarServiceGovernance (governance) |
| **Testnet**  | Not set in config                                                                    | AxelarServiceGovernance (governance) |
| **Mainnet**  | Not set in config                                                                    | AxelarServiceGovernance (governance) |

```bash
# Get AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE="<AXELAR_SERVICE_GOVERNANCE_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c AxelarGasService --action owner

# Transfer ownership to AxelarServiceGovernance
ts-node evm/ownership.js -c AxelarGasService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

### Step 5: Align AxelarGasService Collector Role

New collector: Operators contract.

| Network  | Current Collector                                                                                  | Target Address          |
| -------- | -------------------------------------------------------------------------------------------------- | ----------------------- |
| **Devnet-Amplifier**   | Not set in config | Operators contract      |
| **Stagenet** | `0xc5C525B7Bb2a7Ce95C13Ee5aBdB7F8fd3cb77392`                                   | Operators contract      |
| **Testnet**  | `0x7F83F5cA2AE4206AbFf8a3C3668e88ce5F11C0B5` (per-chain) | Operators contract      |
| **Mainnet**  | `0x7DdB2d76b80B0AA19bDEa48EB1301182F4CeefbC`, `0xfEF5c90d84a1C93804496f5e7fbf98ec0C85243C` (per-chain) | Operators contract      |

```bash
# Get Operators contract address from config
OPERATORS=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.Operators.address" | tr -d '"')

# Verify current collector via config/on-chain call
GAS_SERVICE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarGasService.address" | tr -d '"')

# Update collector via upgrade (collector is set in constructor, requires upgrade)
ts-node evm/deploy-upgradable.js -c AxelarGasService -m create2 --args "{\"collector\": \"$OPERATORS\"}" --reuseProxy
```

### Step 6: Transfer Operators Owner Role

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

### Step 7: Transfer InterchainTokenService Owner Role

New owner: AxelarServiceGovernance.

| Network  | Current Owner                                 | Target Address                       |
| -------- | --------------------------------------------- | ------------------------------------ |
| **Devnet-Amplifier**   | Not set in config | AxelarServiceGovernance (governance) |
| **Stagenet** | Not set in config | AxelarServiceGovernance (governance) |
| **Testnet**  | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | AxelarServiceGovernance (governance) |
| **Mainnet**  | Not set in config | AxelarServiceGovernance (governance) |

```bash
# Get AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE="<AXELAR_SERVICE_GOVERNANCE_ADDRESS>"

# Verify current owner
ts-node evm/ownership.js -c InterchainTokenService --action owner

# Transfer ownership
ts-node evm/ownership.js -c InterchainTokenService --action transferOwnership --newOwner $AXELAR_SERVICE_GOVERNANCE
```

### Step 8: Transfer / Confirm InterchainTokenService Operator Role

New operator: Rate Limiter EOA.

| Network  | Current Operator                             | Target Address      |
| -------- | -------------------------------------------- | ------------------- |
| **Devnet-Amplifier**   | Not set in config | Rate Limiter EOA    |
| **Stagenet** | Not set in config | Rate Limiter EOA    |
| **Testnet**  | Not set in config | Rate Limiter EOA    |
| **Mainnet**  | Not set in config | Rate Limiter EOA    |

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
- [ ] AxelarGasService collector is set to the Operators contract.
- [ ] Operators owner is transferred to the Relayer Operators EOA.
- [ ] InterchainTokenService owner is transferred to AxelarServiceGovernance.
- [ ] InterchainTokenService operator is transferred to the Rate Limiter EOA.
- [ ] All role transfers are verified on-chain.
- [ ] Contract addresses and roles are updated in `${ENV}.json`.
- [ ] Documentation is updated with new role addresses and reasoning.
