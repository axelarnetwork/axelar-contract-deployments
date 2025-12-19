## Consensus Chains Role Transfers & AxelarServiceGovernance Alignment v1.0.0

|                | **Owner**                              |
| -------------- | -------------------------------------- |
| **Created By** | @blockchainguyy <ayush@interoplabs.io> |
| **Deployment** |                                        |

| **Environment**      | **Chain**           | **Deployment Status** | **Date** |
| -------------------- | ------------------- | --------------------- | -------- |
| **Devnet Amplifier** | `core-avalanche`    | -                     | TBD      |
|                      | `core-ethereum`     | -                     | TBD      |
|                      | `core-optimism`     | -                     | TBD      |
| **Stagenet**         | `avalanche`         | Completed             | 19/12/2025      |
|                      | `fantom`            | Completed             | 19/12/2025      |
|                      | `moonbeam`          | -                     | TBD      |
|                      | `kava`              | Completed             | 19/12/2025      |
|                      | `ethereum-sepolia`  | Completed             | 19/12/2025      |
|                      | `arbitrum-sepolia`  | Completed             | 19/12/2025      |
|                      | `linea-sepolia`     | Completed             | 19/12/2025      |
|                      | `polygon-sepolia`   | Completed             | 19/12/2025      |
|                      | `base-sepolia`      | Completed             | 19/12/2025      |
|                      | `blast-sepolia`     | -                     | TBD      |
|                      | `fraxtal`           | -                     | TBD      |
|                      | `mantle-sepolia`    | Completed             | 19/12/2025      |
|                      | `optimism-sepolia`  | Completed             | 19/12/2025      |
| **Testnet**          | `ethereum-sepolia`  | -                     | TBD      |
|                      | `avalanche`         | -                     | TBD      |
|                      | `fantom`            | -                     | TBD      |
|                      | `moonbeam`          | -                     | TBD      |
|                      | `binance`           | -                     | TBD      |
|                      | `kava`              | -                     | TBD      |
|                      | `filecoin-2`        | -                     | TBD      |
|                      | `scroll`            | -                     | TBD      |
|                      | `immutable`         | -                     | TBD      |
|                      | `arbitrum-sepolia`  | -                     | TBD      |
|                      | `centrifuge-2`      | -                     | TBD      |
|                      | `fraxtal`           | -                     | TBD      |
|                      | `optimism-sepolia`  | -                     | TBD      |
|                      | `base-sepolia`      | -                     | TBD      |
|                      | `blast-sepolia`     | -                     | TBD      |
|                      | `mantle-sepolia`    | -                     | TBD      |
|                      | `polygon-sepolia`   | -                     | TBD      |
|                      | `linea-sepolia`     | -                     | TBD      |
| **Mainnet**          | `celo`              | -                     | TBD      |
|                      | `ethereum`          | -                     | TBD      |
|                      | `avalanche`         | -                     | TBD      |
|                      | `fantom`            | -                     | TBD      |
|                      | `polygon`           | -                     | TBD      |
|                      | `moonbeam`          | -                     | TBD      |
|                      | `binance`           | -                     | TBD      |
|                      | `arbitrum`          | -                     | TBD      |
|                      | `kava`              | -                     | TBD      |
|                      | `filecoin`          | -                     | TBD      |
|                      | `optimism`          | -                     | TBD      |
|                      | `linea`             | -                     | TBD      |
|                      | `base`              | -                     | TBD      |
|                      | `mantle`            | -                     | TBD      |
|                      | `scroll`            | -                     | TBD      |
|                      | `centrifuge`        | -                     | TBD      |
|                      | `immutable`         | -                     | TBD      |
|                      | `fraxtal`           | -                     | TBD      |
|                      | `blast`             | -                     | TBD      |

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
| Operators               | owner       | EOA                            | `addOperator`, `removeOperator`, `transferOwnership`, `proposeOwnership`                           | Operators Owner EOA      | Operational registry management for relayer operators                                    |
| Operators               | operators   | EOAs (see config)              | `executeContract`                                                                                  | Relayer broadcaster EOAs | Frequent operational execution of contracts with operator privileges                     |
| InterchainTokenService  | deployer    | EOA                            | -                                                                                                  | -                        | Informational only (initial deployer EOA, no on-chain role to transfer)                  |
| InterchainTokenService  | owner       | EOA (per-chain, see config)    | `setTrustedAddress`, `removeTrustedAddress`, `setPauseStatus`, `migrateInterchainToken`, `upgrade` | AxelarServiceGovernance  | Operational token service management and upgrade control                                 |
| InterchainTokenService  | operator    | EOA                            | `setFlowLimits`, `transferOperatorship`, `proposeOperatorship`                                     | Rate Limiter EOA         | Operational flow limit management for cross-chain token flows                            |

## Pre-requisites

| Network              | Chains                                                                                                                                                                                                                                                           |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet Amplifier** | `core-avalanche,core-ethereum,core-optimism`                                                                                                                                                                                                                     |
| **Stagenet**         | `avalanche,fantom,moonbeam,kava,ethereum-sepolia,arbitrum-sepolia,linea-sepolia,polygon-sepolia,base-sepolia,blast-sepolia,fraxtal,mantle-sepolia,optimism-sepolia`                                                                                              |
| **Testnet**          | `ethereum-sepolia,avalanche,fantom,moonbeam,binance,kava,filecoin-2,scroll,immutable,arbitrum-sepolia,centrifuge-2,fraxtal,optimism-sepolia,base-sepolia,blast-sepolia,mantle-sepolia,polygon-sepolia,linea-sepolia`                                             |
| **Mainnet**          | `celo,ethereum,avalanche,fantom,polygon,moonbeam,binance,arbitrum,kava,filecoin,optimism,linea,base,mantle,scroll,centrifuge,immutable,fraxtal,blast`                                                                                                            |

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

| Network              | `minimumTimeDelay` | `deployer`                                   | `salt`                    | `deploymentMethod` | `operatorAddress (AxelarServiceGovernance Operator EOA)` |
| -------------------- | ------------------ | -------------------------------------------- | ------------------------- | ------------------ | -------------------------------------------------------- |
| **Devnet-amplifier** | `0`                | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `v6.0.4 devnet-amplifier` | `create2`          | `0xD3Ba43B92cED452D04B20710C4db627667476024`             |           
| **Stagenet**         | `300`              | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `v6.0.4`                  | `create2`          | `0x466548FaD128a4A7e1B4D51322061F270bb756DF`             |
| **Testnet**          | `300`              | `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` | `v6.0.4`                  | `create3`          | `TBD`                                                    |
| **Mainnet**          | `259200`           | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | `v6.0.4`                  | `create3`          | `TBD`                                                    |

#### Add AxelarServiceGovernance config to `${ENV}.json`

For each consensus chain where AxelarServiceGovernance is not deployed, add the following configuration:

```json
{
  "AxelarServiceGovernance": {
    "governanceChain": "Axelarnet",
    "governanceAddress": "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj",
    "minimumTimeDelay": [minimumTimeDelay],
    "operator": "[operatorAddress]",
    "deploymentMethod": "[deploymentMethod]",
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
ts-node evm/ownership.js -c AxelarServiceGovernance --action owner
```

```bash
# Verify AxelarServiceGovernance constructor / implementation via explorer
ts-node evm/verify-contract.js -c AxelarServiceGovernance --dir /path/to/axelar-gmp-sdk-solidity
```

### Step 2: Transfer AxelarGateway Governance Role

| Network              | Current Governance                                                                         | Axelar Service Governance |
| -------------------- | ------------------------------------------------------------------------------------------ | ------------------------- |
| **Devnet-Amplifier** | `0xfB71a4d90c37C9BCE7cD4Cb692cE26EA3AC0A319`, `0x677c130e0f17F91F7361AcC15b8d7e6A3D6ECeeb` | TBD                       |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                               | `0x7B1cfbC6F267494f1A187C134E14A2B34CC3C550` |
| **Testnet**          | `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`                                               | TBD                       |
| **Mainnet**          | `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`                                               | TBD                       |

```bash
# Get the AxelarServiceGovernance contract address for this environment
AXELAR_SERVICE_GOVERNANCE=$(cat "./axelar-chains-config/info/$ENV.json" | jq ".chains[\"$CHAIN\"].contracts.AxelarServiceGovernance.address" | tr -d '"')

# Stagenet/devnet
ts-node evm/gateway.js --action transferGovernance --destination $AXELAR_SERVICE_GOVERNANCE

- Note: Specify all the chain names on -n flag where we want proposal to run

# Mainnet/Testnet

# Transfer governance to AxelarServiceGovernance
ts-node evm/gateway.js \
  --action transferGovernance \
  --destination "$AXELAR_SERVICE_GOVERNANCE" \
  --governance \
  --governanceContract InterchainGovernance \
  --activationTime $minimumTimeDelay

# After minimumTimeDelay has passed, execute the proposal
# Use the target and calldata values printed by the schedule command above
ts-node evm/governance.js execute \
  --contractName InterchainGovernance \
  --target <TARGET> \
  --calldata <CALLDATA> \
  --parallel

# Verify governance is now AxelarServiceGovernance
ts-node evm/gateway.js -e $ENV -n $CHAIN --action governance

# Verify transfer completed successfully 
ts-node evm/gateway.js --action governance --parallel
```

### Step 3: Align AxelarGateway MintLimiter to Rate Limiter EOA

New mintLimiter: Rate Limiter EOA

| Network              | Current MintLimiter                                                                        | Rate Limiter EOA                             |
| -------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------- |
| **Devnet-Amplifier** | `0x3EE3DeA54E32B234Fd681509A19155978d1a3D18`, `0xD684531104B38326f41f144b7e710C1707E240F2` | `0xD3Ba43B92cED452D04B20710C4db627667476024` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E`                                               | `0xEd18375d60E7ba8242bD22863796886bE00d9D79` |
| **Testnet**          | `0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68`                                               | TBD                                          |
| **Mainnet**          | `0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68`                                               | TBD                                          |

```bash
# Get Rate Limiter EOA for current environment
MINT_LIMITER="<RATE_LIMITER_EOA>"

# Verify current mintLimiter
ts-node evm/gateway.js --action mintLimiter

# Transfer mintLimiter role
ts-node evm/gateway.js --action transferMintLimiter --destination $MINT_LIMITER --parallel

# Verify transfer completed successfully
ts-node evm/gateway.js --action mintLimiter --parallel
```

### Step 4: Transfer AxelarGasService Owner Role

New owner: AxelarServiceGovernance.

| **Network**          | Current Owner                                | Axelar Service Governance |
| -------------------- | -------------------------------------------- | ------------------------- |
| **Devnet-Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD                       |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `0x7B1cfbC6F267494f1A187C134E14A2B34CC3C550` |
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

New owner: Operators Owner EOA

| **Network**          | Current Owner                                | Operators Owner EOA                          |
| -------------------- | -------------------------------------------- | -------------------------------------------- |
| **Devnet-Amplifier** | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | `0xD3Ba43B92cED452D04B20710C4db627667476024` |
| **Stagenet**         | `0x9f5CDBc370B00C0dF52cf2619FA95907508108df` | `0x0F4fbE61828fd86Ad74D8EF2ed7A0b074ee72B28` |
| **Testnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                                          |
| **Mainnet**          | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | TBD                                          |

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

| Network              | Current Owner                                | Axelar Service Governance |
| -------------------- | -------------------------------------------- | ------------------------- |
| **Devnet-Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | TBD                       |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `0x7B1cfbC6F267494f1A187C134E14A2B34CC3C550` |
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

New operator: Rate Limiter EOA.

| Network              | Current Operator                             | Rate Limiter EOA                             |
| -------------------- | -------------------------------------------- | -------------------------------------------- |
| **Devnet-Amplifier** | `0xba76c6980428A0b10CFC5d8ccb61949677A61233` | `0xD3Ba43B92cED452D04B20710C4db627667476024` |
| **Stagenet**         | `0xBeF25f4733b9d451072416360609e5A4c115293E` | `0xEd18375d60E7ba8242bD22863796886bE00d9D79` |
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
- [ ] Operators owner is transferred to the Operators Owner EOA.
- [ ] InterchainTokenService owner is transferred to AxelarServiceGovernance.
- [ ] InterchainTokenService operator is transferred to the Rate Limiter EOA.
- [ ] All role transfers are verified on-chain.
- [ ] Contract addresses and roles are updated in `${ENV}.json`.
- [ ] Documentation is updated with new role addresses and reasoning.
