# Axelar Amplifier Role Transfers

|                | **Owner**                          |
| -------------- | ---------------------------------- |
| **Created By** | @sean329 <sean.xu@interoplabs.io> |
| **Deployment** |                                    |

| **Environment**      | **Network** | **Deployment Status** | **Date** |
| -------------------- | ----------- | --------------------- | -------- |
| **Devnet Amplifier** | `axelar`    | -                     | TBD      |
| **Stagenet**         | `axelar`    | -                     | TBD      |
| **Testnet**          | `axelar`    | -                     | TBD      |
| **Mainnet**          | `axelar`    | -                     | TBD      |

## Background

Rotate non-critical roles to appropriate operational addresses, and assign critical roles to Axelar Governance Module. This enforces correct permissions, separation of duties, and stronger security for the Axelar Amplifier ecosystem.

**Deployment Note:** Axelar Amplifier contracts are deployed across all environments (Devnet Amplifier, Stagenet, Testnet, and Mainnet). This release document covers role transfers for all four environments.

### Role Transfer Summary

| Contract                    | Role                | Current Role Owner | Operations                                                                                                      | Assign To                    | Reasoning                                                                                   |
| --------------------------- | ------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------- |
| ServiceRegistry             | governanceAccount   | EOA                | Register service, Update service, Override service params, Authorize/Unauthorize verifiers, Jail verifiers      | Axelar Governance Module     | CRITICAL SECURITY - Service registration and verifier management require community consensus |
| Router                      | adminAddress        | EOA                | Freeze/Unfreeze chain, Disable/Enable routing (emergency killswitch)                                           | Emergency Operator EOA       | EMERGENCY RESPONSE - Chain freezing and routing killswitch need rapid response              |
| Router                      | governanceAddress   | EOA                | Register chain, Upgrade gateway                                                                                 | Axelar Governance Module     | PROTOCOL GOVERNANCE - Chain registration and gateway upgrades require community voting      |
| Rewards                     | governanceAddress   | EOA                | Create pool, Update pool parameters                                                                             | Axelar Governance Module     | PROTOCOL GOVERNANCE - Non-urgent pool management operations                                 |
| Coordinator                 | governanceAddress   | EOA                | Register protocol, Register chain, Instantiate chain contracts, Register deployment                             | Axelar Governance Module     | PROTOCOL GOVERNANCE - Protocol-level configuration requires governance                      |
| Multisig                    | adminAddress        | EOA                | Unauthorize callers, Disable/Enable signing (emergency)                                                         | Emergency Operator EOA       | EMERGENCY RESPONSE - Emergency disable signing needs rapid response                         |
| Multisig                    | governanceAddress   | EOA                | Authorize/Unauthorize callers, Disable/Enable signing                                                           | Axelar Governance Module     | PROTOCOL GOVERNANCE - Caller authorization primarily governance, but admin can also act     |
| MultisigProver (All Chains) | governanceAddress   | EOA                | Update signing threshold, Update admin, Update verifier set                                                     | Axelar Governance Module     | CRITICAL SECURITY - Threshold and admin changes are security parameters requiring governance |
| MultisigProver (All Chains) | adminAddress        | EOA                | Update verifier set                                                                                             | Key Rotation EOA             | OPERATIONAL MANAGEMENT - Verifier set updates may need timely response                      |
| VotingVerifier (All Chains) | governanceAddress   | EOA                | Update voting threshold                                                                                         | Axelar Governance Module     | CRITICAL SECURITY - Voting threshold is a critical security parameter                       |
| InterchainTokenService      | governanceAddress   | EOA                | Register/Update chains, Freeze/Unfreeze chain, Disable/Enable execution                                        | Axelar Governance Module     | PROTOCOL GOVERNANCE - Chain registration requires governance                                |
| InterchainTokenService      | adminAddress        | EOA                | Freeze/Unfreeze chain, Disable/Enable execution, Register P2P token instance, Modify supply                    | Emergency Operator EOA       | EMERGENCY RESPONSE - Emergency operations and supply management need rapid response         |
| InterchainTokenService      | operator            | N/A                | Register P2P token instance, Modify supply                                                                      | Relayer Operators EOA        | OPERATIONAL MANAGEMENT - P2P token and supply management                                    |
| XrplVotingVerifier          | governanceAddress   | EOA                | Update voting threshold, Enable/Disable execution, Update admin                                                 | Axelar Governance Module     | CRITICAL SECURITY - Governance has superset of admin permissions, including threshold changes|
| XrplVotingVerifier          | adminAddress        | EOA                | Enable/Disable execution, Update admin                                                                          | Emergency Operator EOA       | EMERGENCY RESPONSE - Emergency pause/unpause needs rapid response                           |
| XrplGateway                 | governanceAddress   | EOA                | Register token metadata, Register local/remote token, Link token, Deploy remote token, Enable/Disable execution, Update admin | Axelar Governance Module | PROTOCOL GOVERNANCE - Token registration and deployment operations                          |
| XrplGateway                 | adminAddress        | EOA                | All governance operations                                                                                       | Emergency Operator EOA       | EMERGENCY RESPONSE - Admin needed for fast response on enable/disable execution             |
| XrplMultisigProver          | governanceAddress   | EOA                | Update signing threshold, Update verifier set, Trust set, Update fee reserve, Update XRPL transaction fee, Update XRPL reserves, Enable/Disable execution, Update admin | Axelar Governance Module | CRITICAL SECURITY - Governance has superset of admin permissions, including threshold changes|
| XrplMultisigProver          | adminAddress        | EOA                | Update verifier set, Trust set, Update fee reserve, Update XRPL transaction fee, Update XRPL reserves, Enable/Disable execution, Update admin | Emergency Operator EOA | EMERGENCY RESPONSE - Operational management where either admin or governance can act       |

**Notes:**
- **AxelarnetGateway**: This contract only has a `nexus` parameter and does not require governance or admin roles. No action needed.
- **NexusGateway**: This contract has been deprecated and is no longer in use. No action needed.
- **Controller and Governance Multisig**: The role assignment strategy for these is still to be determined (TBD) and will be addressed in a future release once finalized.
- **Future Change - XrplMultisigProver**: The `UpdateVerifierSet` operation in the admin role will be moved to a dedicated Key Rotation role in a future contract upgrade, allowing the Key Rotation EOA to handle verifier set updates independently from other admin operations.

## Prerequisites

### Critical: Contract Upgrade Required Before Role Transfers

**Some Amplifier contracts currently lack the `UpdateAdmin` message handler.** Before executing the admin role transfers described in this document, the following contracts must be upgraded to include the required functionality:

| Contract | `UpdateAdmin` Available? | Action Required |
|----------|-------------------------|-----------------|
| Router | No | Upgrade required |
| Multisig | No | Upgrade required |
| InterchainTokenService | No | Upgrade required |
| MultisigProver | Yes (but requires Governance, not Admin) | See note below |
| XRPL Contracts | Yes | N/A |

**Note on MultisigProver**: The `UpdateAdmin` message exists but requires **Governance permission**, not Admin permission. The document's Step 5 should use a governance proposal instead of direct admin execution.

**Note on InterchainTokenService**: This contract requires both `UpdateAdmin` (for Step 6) and `SetOperator` (for Step 7) message handlers to be implemented.

### Governance Roles: No Transfer Needed

**The `governanceAddress` for most contracts is already set to the Axelar Governance Module** (`axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` for mainnet/testnet/stagenet, `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` for devnet-amplifier). No governance role transfers are required.

Additionally, **most contracts do not have an `UpdateGovernance` message handler**. Implementing this functionality is not a current priority since governance roles are already correctly assigned.

### Summary of What Can Be Executed Today

- **Step 2 (Governance Transfers)**: NOT NEEDED - governance roles are already correctly set
- **Steps 3-8 (Admin Transfers)**: BLOCKED - requires contract upgrades to add `UpdateAdmin` functionality

### Environment Setup

Set up your environment for Axelar operations:

```bash
# Set environment variables
export ENV=<devnet-amplifier|stagenet|testnet|mainnet>

# Set chain-id based on environment
case $ENV in
  mainnet)
    export CHAIN_ID="axelar-dojo-1"
    ;;
  testnet)
    export CHAIN_ID="axelar-testnet-lisbon-3"
    ;;
  stagenet)
    export CHAIN_ID="axelar-stagenet-1"
    ;;
  devnet-amplifier)
    export CHAIN_ID="devnet-amplifier"
    ;;
esac

# Ensure you have axelard CLI installed and configured
axelard version

# Set up your key
axelard keys add <key-name> --recover  # if importing existing key
# or
axelard keys add <key-name>  # to create new key

# For scripts that use ts-node (submit-proposal.js, contract.ts, query.ts)
# Set mnemonic for signing transactions
export MNEMONIC="<your-mnemonic-phrase>"

# Set Axelar RPC endpoint
export AXELAR_RPC="https://axelar-rpc.polkachu.com"  # or your preferred RPC endpoint
```

Chain config should exist under `${ENV}.json` file.

## Current Role Owners

### All Environments

| Contract                    | Role              | Devnet Amplifier                             | Stagenet                                     | Testnet                                      | Mainnet                                      |
| --------------------------- | ----------------- | -------------------------------------------- | -------------------------------------------- | -------------------------------------------- | -------------------------------------------- |
| ServiceRegistry             | governanceAccount | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| Router                      | adminAddress      | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |
| Router                      | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| Rewards                     | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| Coordinator                 | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| Multisig                    | adminAddress      | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |
| Multisig                    | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| MultisigProver (All)        | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| MultisigProver (All)        | adminAddress      | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |
| VotingVerifier (All)        | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| InterchainTokenService      | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| InterchainTokenService      | adminAddress      | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` |
| XrplVotingVerifier          | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| XrplVotingVerifier          | adminAddress      | `axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs` | `axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |
| XrplGateway                 | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| XrplGateway                 | adminAddress      | `axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs` | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |
| XrplMultisigProver          | governanceAddress | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| XrplMultisigProver          | adminAddress      | `axelar1lsasewgqj7698e9a25v3c9kkzweee9cvejq5cs` | `axelar1l7vz4m5g92kvga050vk9ycjynywdlk4zhs07dv` | `axelar17qafmnc4hrfa96cq37wg5l68sxh354pj6eky35` | `axelar1pczf792wf3p3xssk4dmwfxrh6hcqnrjp70danj` |
| Governance Multisig         | signers           | See below                                    | See below                                    | See below                                    | See below                                    |
| Controller                  | -                 | `axelar10dwjv6xj0flfj5m3ce4t5v7xpjmrt5du7h6zs6` | `axelar1z5fkx8jt4qthpg5dm0vwgluehuf295jgay6fs5` | `axelar1tf298zq9fn0rjlj23dmw04jfpu2whyrqsch5qn` | `axelar1s952p4ye4hs24hqtnwjpggl0akzpcd5uany5rw` |

### Governance Multisig Signers

**Devnet Amplifier:**
```yaml
public_keys:
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: AsphpgV7Lf7PB53R2XhPu4rjAk0mq8O6/F+uHNWzzgZR
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: AgbSPHIOu18pN4O3ffUD3lKRuqvlZocSlxL8zNfHoleM
```

**Stagenet:**
```yaml
public_keys:
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: AzPKnu1am1+s1o4vsMS03QA6oc/1kTbdHCO4gjmODnGv
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: AxrcLnd9D6ZA3EGdZ9IIrJEx8wUp7JJUj05bFAT9WKdW
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A3Zt7M5XyMG3QVBVhQjPRHP5nvi2IZjV9Ru3T4ozrtM/
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: Ag8xmDDx8roBJArN03oBSaM2SuxgV+4uWfwYlmJ/+zMj
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A6YjBmrjroiQDUdYcgmdUbvK9ZFEPwnpcwImBHXT2oGv
```

**Testnet:**
```yaml
public_keys:
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A0kRUhRv5V/ht0xKWRxRTtPD1QnjPEz9R5/N7PbjbaM/
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: AkfJDWilArWNwP8gmj1Uqg/gnZCfPzDb8gAs9807I4We
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A+CWaOdqcJsE2GJjdLqfUNBT65CNIAqqrbYIsXhiHpE3
```

**Mainnet:**
```yaml
public_keys:
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: Atc53p473TT6qQl0PsaH9p8oEo6hWW95ETA+KjuT4lQt
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A3yUotXxw/YqE6FMuzy37zbT05fo71kPzlQ2GYiZ0KUb
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: AyMZlWEpYSTXronit0uGL5r/NXwozzT6btvg6LLAbf/T
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A/+aBgM++3skGAp1hRk9FBHkcrnx7vBmlH6nX9gmlpyZ
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: A0fY1ohSp9CqaPj8Gl3jv8veCaxGpKnx3iSOlgEtTzXZ
  - '@type': /cosmos.crypto.secp256k1.PubKey
    key: Ay9hXng2C9sg38HZZX+c2e/zzTi+ygtu2ATXcrtrP0xv
```

## Target Role Addresses

Before executing the role transfers, confirm the target addresses for each environment:

| Role Target            | Devnet Amplifier | Stagenet | Testnet | Mainnet |
| ---------------------- | ---------------- | -------- | ------- | ------- |
| Axelar Governance Module | Governance Module | Governance Module | Governance Module | Governance Module |
| Emergency Operator EOA | TBD              | TBD      | TBD     | TBD     |
| Key Rotation EOA       | TBD              | TBD      | TBD     | TBD     |
| Relayer Operators EOA  | TBD              | TBD      | TBD     | TBD     |

**Note:** Axelar Governance Module is the built-in on-chain governance system, not an EOA address. Role transfers to governance will be executed through governance proposals.

## Deployment Steps

### Important: Governance vs Direct Updates

For Axelar Amplifier contracts, role transfers follow two patterns:

1. **Governance Module Assignments**: These require governance proposals and community voting
2. **EOA Assignments**: These can be executed directly by current admin/governance holders

### Step 1 (Optional): Query Current On-Chain Role Info

The following commands can be used to get the current on-chain role information for any contract:

```bash
# Get contract address from config
CONFIG_FILE="./axelar-chains-config/info/$ENV.json"
CONTRACT_ADDRESS=$(jq -r '.axelar.contracts.Router.address' $CONFIG_FILE)

# Query current admin and governance addresses
axelard query wasm contract-state smart $CONTRACT_ADDRESS '{"admin": {}}' --node $AXELAR_RPC
axelard query wasm contract-state smart $CONTRACT_ADDRESS '{"governance_address": {}}' --node $AXELAR_RPC
```

**Query formats by contract:**
- `ServiceRegistry`: `{"governance_account": {}}`
- `Router`, `Multisig`, `MultisigProver`, `ITS`, etc.: `{"admin": {}}` and `{"governance_address": {}}`

### Step 2: Transfer Governance Roles to Axelar Governance Module

> **NO ACTION REQUIRED**: As noted in the Prerequisites section, the `governanceAddress`/`governanceAccount` for all contracts is **already set to the Axelar Governance Module**. Additionally, most contracts do not have an `UpdateGovernance` message handler implemented. Skip this step and proceed to Step 3 (after contract upgrades are completed).

For reference, if governance transfers were needed in the future, they would apply to:

- ServiceRegistry (governanceAccount)
- Router (governanceAddress)
- Rewards (governanceAddress)
- Coordinator (governanceAddress)
- Multisig (governanceAddress)
- MultisigProver for all chains (governanceAddress)
- VotingVerifier for all chains (governanceAddress)
- InterchainTokenService (governanceAddress)
- XrplVotingVerifier (governanceAddress)
- XrplGateway (governanceAddress)
- XrplMultisigProver (governanceAddress)

**Step 2a: Check Current Governance Status**

First, run the following script to identify which contracts need governance transfer:

```bash
#!/bin/bash
# check-governance-status.sh
# Usage: ./check-governance-status.sh <env>
# Example: ./check-governance-status.sh mainnet

ENV=${1:-mainnet}
CONFIG_FILE="./axelar-chains-config/info/$ENV.json"

# Governance Module addresses per environment
case $ENV in
  devnet-amplifier)
    # Verify this on axelarscan to ensure it's still the current Axelar Governance address
    GOVERNANCE_MODULE="axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9"
    ;;
  *)
    GOVERNANCE_MODULE="axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"
    ;;
esac

echo "=== Environment: $ENV ==="
echo "=== Governance Module: $GOVERNANCE_MODULE ==="
echo ""

# Function to check governance status
check_governance() {
  local contract=$1
  local address=$2
  local gov_field=$3
  
  if [ -z "$address" ] || [ "$address" == "null" ]; then
    return
  fi
  
  local current_gov=$(jq -r ".axelar.contracts.$contract.$gov_field // empty" $CONFIG_FILE)
  
  if [ "$current_gov" == "$GOVERNANCE_MODULE" ]; then
    echo "[OK] $contract: Already set to Governance Module"
  elif [ -n "$current_gov" ]; then
    echo "[NEEDS TRANSFER] $contract: Needs transfer"
    echo "   Address: $address"
    echo "   Current: $current_gov"
    echo "   Command: ts-node cosmwasm/submit-proposal.js executeByGovernance -e $ENV -c $contract -t \"Transfer $contract governance\" -d \"Transfer governance to Governance Module\" --msg '{\"update_governance\":{\"governance\":\"$GOVERNANCE_MODULE\"}}'"
    echo ""
  fi
}

# Function to check chain-specific contracts
check_chain_governance() {
  local contract=$1
  local chain=$2
  local gov_field=$3
  
  local address=$(jq -r ".axelar.contracts.$contract[\"$chain\"].address // empty" $CONFIG_FILE)
  local current_gov=$(jq -r ".axelar.contracts.$contract[\"$chain\"].$gov_field // empty" $CONFIG_FILE)
  
  if [ -z "$address" ] || [ "$address" == "null" ]; then
    return
  fi
  
  if [ "$current_gov" == "$GOVERNANCE_MODULE" ]; then
    echo "[OK] $contract[$chain]: Already set to Governance Module"
  elif [ -n "$current_gov" ]; then
    echo "[NEEDS TRANSFER] $contract[$chain]: Needs transfer"
    echo "   Address: $address"
    echo "   Current: $current_gov"
    echo "   Command: ts-node cosmwasm/submit-proposal.js executeByGovernance -e $ENV -c $contract -n $chain -t \"Transfer $contract[$chain] governance\" -d \"Transfer governance to Governance Module\" --msg '{\"update_governance\":{\"governance\":\"$GOVERNANCE_MODULE\"}}'"
    echo ""
  fi
}

echo "=== Checking Global Contracts ==="
echo ""

# Global contracts
for contract in ServiceRegistry Router Rewards Coordinator Multisig InterchainTokenService XrplVotingVerifier XrplGateway XrplMultisigProver; do
  address=$(jq -r ".axelar.contracts.$contract.address // empty" $CONFIG_FILE)
  
  # ServiceRegistry uses governanceAccount, others use governanceAddress
  if [ "$contract" == "ServiceRegistry" ]; then
    check_governance "$contract" "$address" "governanceAccount"
  else
    check_governance "$contract" "$address" "governanceAddress"
  fi
done

echo ""
echo "=== Checking Per-Chain Contracts ==="
echo ""

# Get all chain names from MultisigProver
chains=$(jq -r '.axelar.contracts.MultisigProver | keys[] | select(. != "address" and . != "codeId" and . != "governanceAddress" and . != "adminAddress" and . != "lastCodeId" and . != "codeHash" and . != "storeCodeProposalId" and . != "storeCodeProposalCodeHash")' $CONFIG_FILE 2>/dev/null)

for chain in $chains; do
  check_chain_governance "MultisigProver" "$chain" "governanceAddress"
  check_chain_governance "VotingVerifier" "$chain" "governanceAddress"
done

echo ""
echo "=== Done ==="
```

**Step 2b: Submit Governance Proposals**

For each contract that needs governance transfer, use `submit-proposal.js executeByGovernance`:

```bash
# Set environment
ENV=mainnet  # or testnet, stagenet, devnet-amplifier

# For global contracts (e.g., Router, Multisig, InterchainTokenService)
ts-node cosmwasm/submit-proposal.js executeByGovernance \
  -e $ENV \
  -c Router \
  -t "Transfer Router Governance to Governance Module" \
  -d "This proposal transfers the governance role of Router to the Axelar Governance Module for proper decentralized governance." \
  --msg '{"update_governance":{"governance":"axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"}}'

# For chain-specific contracts (e.g., MultisigProver, VotingVerifier)
ts-node cosmwasm/submit-proposal.js executeByGovernance \
  -e $ENV \
  -c MultisigProver \
  -n flow \
  -t "Transfer MultisigProver[flow] Governance to Governance Module" \
  -d "This proposal transfers the governance role of MultisigProver for flow chain to the Axelar Governance Module." \
  --msg '{"update_governance":{"governance":"axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"}}'

# For ServiceRegistry (uses governanceAccount instead of governanceAddress)
ts-node cosmwasm/submit-proposal.js executeByGovernance \
  -e $ENV \
  -c ServiceRegistry \
  -t "Transfer ServiceRegistry Governance to Governance Module" \
  -d "This proposal transfers the governance role of ServiceRegistry to the Axelar Governance Module." \
  --msg '{"update_governance_account":{"governance_account":"axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"}}'
```

**Note for devnet-amplifier:** Use `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` as the governance module address.

**Step 2b.1: Vote on Proposals (Non-mainnet environments only)**

For testnet, stagenet, and devnet-amplifier, you can vote on your own proposals to make them pass:

```bash
# Query the proposal ID (get the latest proposal)
PROPOSAL_ID=$(axelard query gov proposals --status voting_period --output json | jq -r '.proposals[-1].id')

# Vote yes on the proposal
axelard tx gov vote $PROPOSAL_ID yes \
  --from <your_key> \
  --chain-id $CHAIN_ID \
  --gas auto \
  --gas-adjustment 1.5 \
  --gas-prices 0.00005uaxl \
  -y

# Wait for voting period to end (check proposal status)
axelard query gov proposal $PROPOSAL_ID
```

**Note:** On mainnet, proposals require community voting and will take the full voting period (typically 3-7 days).

**Step 2c: Verify Governance Transfers**

After proposals pass, verify the transfers:

```bash
# Get contract addresses from config
CONFIG_FILE="./axelar-chains-config/info/$ENV.json"
ROUTER=$(jq -r '.axelar.contracts.Router.address' $CONFIG_FILE)
MULTISIG=$(jq -r '.axelar.contracts.Multisig.address' $CONFIG_FILE)
ITS=$(jq -r '.axelar.contracts.InterchainTokenService.address' $CONFIG_FILE)

# Query governance address for global contracts
axelard query wasm contract-state smart $ROUTER '{"governance_address":{}}' --node $AXELAR_RPC
axelard query wasm contract-state smart $MULTISIG '{"governance_address":{}}' --node $AXELAR_RPC
axelard query wasm contract-state smart $ITS '{"governance_address":{}}' --node $AXELAR_RPC

# For chain-specific contracts (example: flow)
CHAIN_NAME=flow
MULTISIG_PROVER=$(jq -r ".axelar.contracts.MultisigProver[\"$CHAIN_NAME\"].address" $CONFIG_FILE)
VOTING_VERIFIER=$(jq -r ".axelar.contracts.VotingVerifier[\"$CHAIN_NAME\"].address" $CONFIG_FILE)

axelard query wasm contract-state smart $MULTISIG_PROVER '{"governance_address":{}}' --node $AXELAR_RPC
axelard query wasm contract-state smart $VOTING_VERIFIER '{"governance_address":{}}' --node $AXELAR_RPC
```

**Note:** `contract-admin` queries `admin` address, not `governance` address. Use raw axelard queries above to verify governance transfers.

**Important:** If the governance address is not set to the Governance Module after the proposal passes, **STOP** and investigate before proceeding to the next steps. Possible causes:
- Proposal did not pass (check proposal status with `axelard query gov proposal <proposal_id>`)
- Proposal passed but not yet executed (wait for execution)
- Wrong contract address or message format in the proposal

Do not proceed with admin transfers until all governance transfers are verified.

### Step 3: Transfer Router Admin to Emergency Operator EOA

> **BLOCKED**: Router contract does not currently have `UpdateAdmin` message handler. Contract upgrade required before this step can be executed.

The Router admin role should be transferred to an Emergency Operator EOA for rapid response capabilities.

**New Admin**: Emergency Operator EOA - refer to the Target Role Addresses Table above

```bash
# Set variables
ENV=mainnet  # or testnet, stagenet, devnet-amplifier
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>

# Get contract address from config
ROUTER_CONTRACT=$(jq -r '.axelar.contracts.Router.address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update (requires current admin key)
axelard tx wasm execute $ROUTER_CONTRACT \
  "{\"update_admin\": {\"admin\": \"$EMERGENCY_OPERATOR_EOA\"}}" \
  --from <current_admin_key> \
  --chain-id $CHAIN_ID \
  --gas auto \
  --gas-adjustment 1.5 \
  --gas-prices 0.00005uaxl \
  -y

# Verify transfer using query script
ts-node cosmwasm/query.ts contract-admin -c Router -e $ENV
```

**Important:** Verify the output shows `$EMERGENCY_OPERATOR_EOA` as the admin. If not, check the transaction status and do not proceed until verified.

### Step 4: Transfer Multisig Admin to Emergency Operator EOA

> **BLOCKED**: Multisig contract does not currently have `UpdateAdmin` message handler. Contract upgrade required before this step can be executed.

**New Admin**: Emergency Operator EOA - refer to the Target Role Addresses Table above

```bash
# Set variables
ENV=mainnet
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>

# Get contract address from config
MULTISIG_CONTRACT=$(jq -r '.axelar.contracts.Multisig.address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update (requires current admin key)
axelard tx wasm execute $MULTISIG_CONTRACT \
  "{\"update_admin\": {\"admin\": \"$EMERGENCY_OPERATOR_EOA\"}}" \
  --from <current_admin_key> \
  --chain-id $CHAIN_ID \
  --gas auto \
  --gas-adjustment 1.5 \
  --gas-prices 0.00005uaxl \
  -y

# Verify transfer using query script
ts-node cosmwasm/query.ts contract-admin -c Multisig -e $ENV
```

**Important:** Verify the output shows `$EMERGENCY_OPERATOR_EOA` as the admin. If not, check the transaction status and do not proceed until verified.

### Step 5: Transfer MultisigProver Admin to Key Rotation EOA

The MultisigProver admin role (for all supported chains) should be transferred to a Key Rotation EOA for timely verifier set updates.

> **IMPORTANT**: Unlike other contracts, MultisigProver's `update_admin` message **requires Governance permission**, not Admin permission. This must be executed via governance proposal.

**New Admin**: Key Rotation EOA - refer to the Target Role Addresses Table above

```bash
# Set variables
ENV=mainnet
KEY_ROTATION_EOA=<KEY_ROTATION_EOA_ADDRESS>
PROPOSAL_FILE=multisig_prover_admin_transfer.json

# Set deposit based on environment (expedited proposals require higher deposit)
case $ENV in
  mainnet|testnet)
    DEPOSIT="3000000000uaxl"  # 3000 AXL for expedited on mainnet/testnet
    ;;
  stagenet)
    DEPOSIT="150000000uaxl"   # 150 AXL for expedited on stagenet
    ;;
  devnet-amplifier)
    DEPOSIT="150000000uamplifier"  # 150 for expedited on devnet-amplifier
    ;;
esac

# Step 1: Generate proposal JSON for all chains using --dry-run
echo "Generating proposal messages for all MultisigProver contracts..."

# Start JSON structure
echo '{
  "title": "Transfer MultisigProver Admin to Key Rotation EOA",
  "summary": "Transfer admin role of MultisigProver for all chains to Key Rotation EOA for timely verifier set updates",
  "expedited": true,
  "messages": [' > $PROPOSAL_FILE

FIRST=true
for CHAIN_NAME in flow sui stellar xrpl-evm plume hedera berachain hyperliquid monad; do
  MULTISIG_PROVER_CONTRACT=$(jq -r ".axelar.contracts.MultisigProver[\"$CHAIN_NAME\"].address // empty" ./axelar-chains-config/info/$ENV.json)
  
  if [ -n "$MULTISIG_PROVER_CONTRACT" ] && [ "$MULTISIG_PROVER_CONTRACT" != "null" ]; then
    echo "  Adding MultisigProver[$CHAIN_NAME]: $MULTISIG_PROVER_CONTRACT"
    
    # Use --dry-run to get the message JSON without submitting
    OUTPUT=$(ts-node cosmwasm/submit-proposal.js executeByGovernance \
      -e $ENV \
      -c MultisigProver \
      -n $CHAIN_NAME \
      -t "placeholder" \
      -d "placeholder" \
      --msg "{\"update_admin\":{\"admin\":\"$KEY_ROTATION_EOA\"}}" \
      --dry-run 2>/dev/null)
    
    if [ -n "$OUTPUT" ] && [ "$OUTPUT" != "[]" ]; then
      if [ "$FIRST" = true ]; then
        FIRST=false
      else
        echo "," >> $PROPOSAL_FILE
      fi
      # Remove outer brackets from array and append
      echo "$OUTPUT" | sed 's/^\[//;s/\]$//' >> $PROPOSAL_FILE
    fi
  fi
done

# Close JSON structure
echo "
  ],
  \"deposit\": \"$DEPOSIT\"
}" >> $PROPOSAL_FILE

echo "Generated $PROPOSAL_FILE"
cat $PROPOSAL_FILE

# Step 2: Submit combined proposal via axelard
echo ""
echo "Submitting combined governance proposal..."
axelard tx gov submit-proposal $PROPOSAL_FILE \
  --from <wallet> \
  --chain-id $CHAIN_ID \
  --node $NODE \
  --gas auto \
  --gas-adjustment 1.4 \
  -y

# Step 3: After proposal passes, verify all transfers
echo ""
echo "After proposal passes, verify transfers with:"
for CHAIN_NAME in flow sui stellar xrpl-evm plume hedera berachain hyperliquid monad; do
  echo "ts-node cosmwasm/query.ts contract-admin -c MultisigProver -n $CHAIN_NAME -e $ENV"
done
```

**Important:** Verify each chain's MultisigProver shows `$KEY_ROTATION_EOA` as the admin. If any chain fails, investigate that specific transaction before proceeding.

### Step 6: Transfer InterchainTokenService Admin to Emergency Operator EOA

> **BLOCKED**: InterchainTokenService contract does not currently have `UpdateAdmin` message handler. Contract upgrade required before this step can be executed.

**New Admin**: Emergency Operator EOA - refer to the Target Role Addresses Table above

```bash
# Set variables
ENV=mainnet
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>

# Get contract address from config
ITS_CONTRACT=$(jq -r '.axelar.contracts.InterchainTokenService.address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update (requires current admin key)
axelard tx wasm execute $ITS_CONTRACT \
  "{\"update_admin\": {\"admin\": \"$EMERGENCY_OPERATOR_EOA\"}}" \
  --from <current_admin_key> \
  --chain-id $CHAIN_ID \
  --gas auto \
  --gas-adjustment 1.5 \
  --gas-prices 0.00005uaxl \
  -y

# Verify transfer using query script
ts-node cosmwasm/query.ts contract-admin -c InterchainTokenService -e $ENV
```

**Important:** Verify the output shows `$EMERGENCY_OPERATOR_EOA` as the admin. If not, check the transaction status and do not proceed until verified.

### Step 7: Set InterchainTokenService Operator to Relayer Operators EOA

> **BLOCKED**: InterchainTokenService contract does not currently have `SetOperator` message handler. Contract upgrade required before this step can be executed.

**New Operator**: Relayer Operators EOA - refer to the Target Role Addresses Table above

```bash
# Set variables
ENV=mainnet
RELAYER_OPERATORS_EOA=<RELAYER_OPERATORS_EOA_ADDRESS>

# Get contract address from config
ITS_CONTRACT=$(jq -r '.axelar.contracts.InterchainTokenService.address' ./axelar-chains-config/info/$ENV.json)

# Set operator (requires current admin or governance key)
axelard tx wasm execute $ITS_CONTRACT \
  "{\"set_operator\": {\"operator\": \"$RELAYER_OPERATORS_EOA\"}}" \
  --from <current_admin_or_governance_key> \
  --chain-id $CHAIN_ID \
  --gas auto \
  --gas-adjustment 1.5 \
  --gas-prices 0.00005uaxl \
  -y

# Verify operator
axelard query wasm contract-state smart $ITS_CONTRACT '{"operator": {}}'
```

### Step 8: Transfer XRPL Contract Admin Roles to Emergency Operator EOA

Transfer admin roles for XrplVotingVerifier, XrplGateway, and XrplMultisigProver to Emergency Operator EOA.

**New Admin**: Emergency Operator EOA - refer to the Target Role Addresses Table above

**XRPL Chain Names by Environment:**

| Environment | XRPL Chain Name |
|-------------|-----------------|
| devnet-amplifier | `xrpl-dev` |
| stagenet | `xrpl` |
| testnet | `xrpl` |
| mainnet | `xrpl` |

```bash
# Set variables
ENV=mainnet
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>

# Set XRPL chain name based on environment
case $ENV in
  devnet-amplifier)
    XRPL_CHAIN_NAME="xrpl-dev"
    ;;
  *)
    XRPL_CHAIN_NAME="xrpl"
    ;;
esac

# Get contract addresses from config
XRPL_VOTING_VERIFIER=$(jq -r '.axelar.contracts.XrplVotingVerifier.address' ./axelar-chains-config/info/$ENV.json)
XRPL_GATEWAY=$(jq -r '.axelar.contracts.XrplGateway.address' ./axelar-chains-config/info/$ENV.json)
XRPL_MULTISIG_PROVER=$(jq -r '.axelar.contracts.XrplMultisigProver.address' ./axelar-chains-config/info/$ENV.json)

# Update all XRPL contract admins
for CONTRACT_ADDR in $XRPL_VOTING_VERIFIER $XRPL_GATEWAY $XRPL_MULTISIG_PROVER; do
  if [ -n "$CONTRACT_ADDR" ] && [ "$CONTRACT_ADDR" != "null" ]; then
    echo "Updating admin for: $CONTRACT_ADDR"
    axelard tx wasm execute $CONTRACT_ADDR \
      "{\"update_admin\": {\"admin\": \"$EMERGENCY_OPERATOR_EOA\"}}" \
      --from <current_admin_key> \
      --chain-id $CHAIN_ID \
      --gas auto \
      --gas-adjustment 1.5 \
      --gas-prices 0.00005uaxl \
      -y
    sleep 6
  fi
done

# Verify all transfers
ts-node cosmwasm/query.ts contract-admin -c XrplVotingVerifier -n $XRPL_CHAIN_NAME -e $ENV
ts-node cosmwasm/query.ts contract-admin -c XrplGateway -n $XRPL_CHAIN_NAME -e $ENV
ts-node cosmwasm/query.ts contract-admin -c XrplMultisigProver -n $XRPL_CHAIN_NAME -e $ENV
```

**Important:** Verify all three XRPL contracts show `$EMERGENCY_OPERATOR_EOA` as the admin. If any contract fails, investigate that specific transaction before marking this step complete.

## Verification Checklist

After completing role transfers, verify all changes:

### Governance Module Assignments

- [ ] ServiceRegistry `governanceAccount` is Governance Module
- [ ] Router `governanceAddress` is Governance Module
- [ ] Rewards `governanceAddress` is Governance Module
- [ ] Coordinator `governanceAddress` is Governance Module
- [ ] Multisig `governanceAddress` is Governance Module
- [ ] All MultisigProver `governanceAddress` is Governance Module
- [ ] All VotingVerifier `governanceAddress` is Governance Module
- [ ] InterchainTokenService `governanceAddress` is Governance Module
- [ ] XrplVotingVerifier `governanceAddress` is Governance Module
- [ ] XrplGateway `governanceAddress` is Governance Module
- [ ] XrplMultisigProver `governanceAddress` is Governance Module

### Emergency Operator EOA Assignments

- [ ] Router `adminAddress` is Emergency Operator EOA
- [ ] Multisig `adminAddress` is Emergency Operator EOA
- [ ] InterchainTokenService `adminAddress` is Emergency Operator EOA
- [ ] XrplVotingVerifier `adminAddress` is Emergency Operator EOA
- [ ] XrplGateway `adminAddress` is Emergency Operator EOA
- [ ] XrplMultisigProver `adminAddress` is Emergency Operator EOA

### Other Role Assignments

- [ ] All MultisigProver `adminAddress` is Key Rotation EOA
- [ ] InterchainTokenService `operator` is Relayer Operators EOA
- [ ] All transfers verified via contract queries
- [ ] Configuration updated in `${ENV}.json` if necessary
- [ ] Documentation updated with new role addresses

## Notes

1. **Governance Module**: The Axelar Governance Module is the built-in on-chain governance system. Role transfers to governance require community proposals and voting.

2. **Dual Role Pattern**: Some contracts (like Multisig, XrplGateway, XrplMultisigProver) have both governance and admin roles where:
   - **Governance**: Has superset of permissions, including critical security parameter changes
   - **Admin**: Has operational permissions for emergency response and day-to-day operations

3. **Emergency Response**: Emergency Operator EOA roles are designated for rapid response scenarios:
   - Router: Freeze/unfreeze chains, disable/enable routing
   - Multisig: Disable/enable signing
   - InterchainTokenService: Freeze/unfreeze chains, emergency pause
   - XRPL Contracts: Emergency pause/unpause operations

4. **Key Rotation**: MultisigProver admin roles are assigned to Key Rotation EOA for timely verifier set updates without requiring full governance process.

5. **Future Change - XrplMultisigProver**: The `UpdateVerifierSet` operation will be migrated to a dedicated Key Rotation role in a future contract upgrade. This will separate verifier set rotation from other admin operations, allowing the Key Rotation EOA to handle it independently while other admin operations remain with the Emergency Operator EOA.

6. **No Action Contracts**: 
   - **AxelarnetGateway**: Only has `nexus` parameter, no governance/admin roles - no action needed
   - **NexusGateway**: Deprecated contract - no action needed
   - **Controller**: Current addresses are documented but role assignment strategy is TBD - will be addressed in future release once finalized
   - **Governance Multisig**: Signers are documented but role assignment strategy is TBD - will be addressed in future release once finalized

7. **Chain-Specific Contracts**: MultisigProver and VotingVerifier are deployed for multiple chains (Flow, Sui, Stellar, XRPL-EVM, Plume, Hedera, Berachain, Hyperliquid, Monad). Each chain's instance requires separate role transfer.

8. **Governance Proposals**: All governance role transfers require:
   - Proposal submission with deposit
   - Voting period (typically 3-7 days)
   - Minimum quorum and approval threshold
   - Proposal execution after passing

9. **Contract Admin vs Governance**: In CosmWasm, there's also a contract-level admin (set during instantiation) that can migrate contracts. This is separate from application-level admin/governance roles discussed in this document.

10. **Testing Recommended**: For testnet and stagenet, consider testing role transfers on a subset of contracts before executing on all contracts and on mainnet.

