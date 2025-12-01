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

Set up your environment for Axelar operations:

```bash
# Set environment variables
export ENV=<devnet-amplifier|stagenet|testnet|mainnet>

# Ensure you have axelard CLI installed and configured
axelard version

# Set up your key
axelard keys add <key-name> --recover  # if importing existing key
# or
axelard keys add <key-name>  # to create new key
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
| Emergency Operator EOA | axelar15kkglkna4jhjtc5vhwla7lmwt2edyv4xg03jl6              | axelar1ygf7zjukhjhhzyjl0w4qta9872zywsqhj9fqh2      | TBD     | TBD     |
| Key Rotation EOA       | axelar1ea7vjq5txwa6uqqgmv3gxv80w6nssvqjtxuk8e              | axelar1yfu0pyp0vhf9n978ch2wwsmhklzqpwy6dmfpfr      | TBD     | TBD     |
| Relayer Operators EOA  | axelar1zu6dcqk3cpww7srg2sz6ft5xa8jfwxnpz0xx2s              | axelar179mu034s39kghzr0dl2yvxqn9zqma7ccs86rhu      | TBD     | TBD     |

**Note:** Axelar Governance Module is the built-in on-chain governance system, not an EOA address. Role transfers to governance will be executed through governance proposals.

## Deployment Steps

### Important: Governance vs Direct Updates

For Axelar Amplifier contracts, role transfers follow two patterns:

1. **Governance Module Assignments**: These require governance proposals and community voting
2. **EOA Assignments**: These can be executed directly by current admin/governance holders

### Step 1: Verify Current Role Owners

Before making any transfers, query the current contract state to verify role owners:

```bash
# Query ServiceRegistry governance account
axelard query wasm contract-state smart <SERVICE_REGISTRY_CONTRACT_ADDRESS> \
  '{"governanceAccount": {}}'

# Query Router admin and governance
axelard query wasm contract-state smart <ROUTER_CONTRACT_ADDRESS> \
  '{"adminAddress": {}}'
axelard query wasm contract-state smart <ROUTER_CONTRACT_ADDRESS> \
  '{"governanceAddress": {}}'

# Query Multisig admin and governance
axelard query wasm contract-state smart <MULTISIG_CONTRACT_ADDRESS> \
  '{"adminAddress": {}}'
axelard query wasm contract-state smart <MULTISIG_CONTRACT_ADDRESS> \
  '{"governanceAddress": {}}'

# Query MultisigProver for a specific chain
axelard query wasm contract-state smart <MULTISIG_PROVER_CONTRACT_ADDRESS> \
  '{"adminAddress": {}}'
axelard query wasm contract-state smart <MULTISIG_PROVER_CONTRACT_ADDRESS> \
  '{"governanceAddress": {}}'

# Query InterchainTokenService admin and governance
axelard query wasm contract-state smart <ITS_CONTRACT_ADDRESS> \
  '{"adminAddress": {}}'
axelard query wasm contract-state smart <ITS_CONTRACT_ADDRESS> \
  '{"governanceAddress": {}}'
```

**Contract Addresses by Environment:**

The following table provides the contract addresses you need for the queries above. Replace the placeholders in the commands with these addresses according to your target environment.

| Contract                             | Devnet Amplifier                                     | Stagenet                                             | Testnet                                              | Mainnet                                              |
| ------------------------------------ | ---------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------- |
| SERVICE_REGISTRY_CONTRACT_ADDRESS    | `axelar1c9fkszt5lq34vvvlat3fxj6yv7ejtqapz04e97vtc9m5z9cwnamq8zjlhz` | `axelar15454y4v8x2ennqq6k0t4cu4r0cpqsy3d6m2jw7d0p4tagaafs29qnlhljd` | `axelar1rpj2jjrv3vpugx9ake9kgk3s2kgwt0y60wtkmcgfml5m3et0mrls6nct9m` | `axelar1rpj2jjrv3vpugx9ake9kgk3s2kgwt0y60wtkmcgfml5m3et0mrls6nct9m` |
| ROUTER_CONTRACT_ADDRESS              | `axelar14jjdxqhuxk803e9pq64w4fgf385y86xxhkpzswe9crmu6vxycezst0zq8y` | `axelar1saq5hcztvd26vvkquv4dzn8e0uu967nuyxtcful3pvv7ajsw4kmq72xft9` | `axelar1d9atnamjjhtc46zmzyc202llqs0rhtxnphs6mkjurekath3mkgtq7hsk93` | `axelar1d9atnamjjhtc46zmzyc202llqs0rhtxnphs6mkjurekath3mkgtq7hsk93` |
| MULTISIG_CONTRACT_ADDRESS            | `axelar19jxy26z0qnnspa45y5nru0l5rmy9d637z5km2ndjxthfxf5qaswst9290r` | `axelar143vjln56ke4pjmj5ut7u3358ywyfl7h5rg58js8gprr39664wcqs72vs3u` | `axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5` | `axelar14a4ar5jh7ue4wg28jwsspf23r8k68j7g5d6d3fsttrhp42ajn4xq6zayy5` |
| MULTISIG_PROVER_CONTRACT_ADDRESS (flow example) | `axelar19ukamzyhhaw4algkqch5relyg0h972smhj8zge2usjp8z7yaezrquc475f` | `axelar1u8cmvyq54fepjlt77vqnwyxhf4rqn5j5axgta7py34zlwue3v5fsy2pvhf` | `axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0` | `axelar1rsuejfntt4rs2y8dn4dd3acszs00zyg9wpnsc6fmhevcp6plu5qspzn7e0` |
| ITS_CONTRACT_ADDRESS                 | `axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4` | `axelar1ph8qufmsh556e40uk0ceaufc06nwhnw0ksgdqqk6ldszxchh8llq8x52dk` | `axelar1aqcj54lzz0rk22gvqgcn8fr5tx4rzwdv5wv5j9dmnacgefvd7wzsy2j2mr` | `axelar1aqcj54lzz0rk22gvqgcn8fr5tx4rzwdv5wv5j9dmnacgefvd7wzsy2j2mr` |

**Note:** MultisigProver addresses are per-chain. The addresses above show the `flow` chain as an example. To query MultisigProver for other chains (sui, stellar, xrpl-evm, plume, hedera, berachain, hyperliquid, monad), replace the contract address with the corresponding chain's MultisigProver address from `axelar-chains-config/info/<environment>.json`.

### Step 2: Transfer Governance Roles to Axelar Governance Module

All `governanceAddress`/`governanceAccount` roles should be transferred to the Axelar Governance Module through governance proposals. This applies to:

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

**Process:**

```bash
# 1. Create governance proposal JSON
cat > update_governance_proposal.json <<EOF
{
  "title": "Transfer [Contract Name] Governance to Governance Module",
  "description": "This proposal transfers the governance role of [Contract Name] to the Axelar Governance Module for proper decentralized governance.",
  "msgs": [
    {
      "@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
      "sender": "<governance_module_address>",
      "contract": "<contract_address>",
      "msg": {
        "update_governance": {
          "governance": "<governance_module_address>"
        }
      },
      "funds": []
    }
  ],
  "deposit": "10000000uaxl"
}
EOF

# Note: The sender must be the governance module address because when a governance 
# proposal executes, the messages are executed BY the governance module itself, 
# not by the proposal submitter.

# 2. Submit the governance proposal
axelard tx gov submit-proposal update_governance_proposal.json \
  --from <your_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# 3. Vote on the proposal (validators and delegators)
axelard tx gov vote <proposal_id> yes \
  --from <your_key> \
  --chain-id <chain_id> \
  --gas auto \
  --fees 5000uaxl

# 4. Wait for voting period to end and proposal to pass

# 5. Verify the governance transfer
axelard query wasm contract-state smart <contract_address> \
  '{"governanceAddress": {}}'
```

### Step 3: Transfer Router Admin to Emergency Operator EOA

The Router admin role should be transferred to an Emergency Operator EOA for rapid response capabilities.

**New Admin**: Emergency Operator EOA - refer to the Target Role Addresses Table above

```bash
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>
ROUTER_CONTRACT=$(jq -r '.axelar.contracts.Router.address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update
axelard tx wasm execute $ROUTER_CONTRACT \
  '{"update_admin": {"admin": "'$EMERGENCY_OPERATOR_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# Verify transfer
axelard query wasm contract-state smart $ROUTER_CONTRACT \
  '{"adminAddress": {}}'
```

### Step 4: Transfer Multisig Admin to Emergency Operator EOA

**New Admin**: Emergency Operator EOA - refer to the Target Role Addresses Table above

```bash
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>
MULTISIG_CONTRACT=$(jq -r '.axelar.contracts.Multisig.address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update
axelard tx wasm execute $MULTISIG_CONTRACT \
  '{"update_admin": {"admin": "'$EMERGENCY_OPERATOR_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# Verify transfer
axelard query wasm contract-state smart $MULTISIG_CONTRACT \
  '{"adminAddress": {}}'
```

### Step 5: Transfer MultisigProver Admin to Key Rotation EOA

The MultisigProver admin role (for all supported chains) should be transferred to a Key Rotation EOA for timely verifier set updates.

**New Admin**: Key Rotation EOA - refer to the Target Role Addresses Table above

```bash
KEY_ROTATION_EOA=<KEY_ROTATION_EOA_ADDRESS>

# For each chain's MultisigProver (flow, sui, stellar, xrpl-evm, plume, hedera, berachain, hyperliquid, monad)
CHAIN_NAME="<chain_name>"  # e.g., "sui", "stellar", etc.
MULTISIG_PROVER_CONTRACT=$(jq -r '.axelar.contracts.MultisigProver["'$CHAIN_NAME'"].address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update
axelard tx wasm execute $MULTISIG_PROVER_CONTRACT \
  '{"update_admin": {"admin": "'$KEY_ROTATION_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# Verify transfer
axelard query wasm contract-state smart $MULTISIG_PROVER_CONTRACT \
  '{"adminAddress": {}}'
```

Repeat this step for all chains with MultisigProver deployments.

### Step 6: Transfer InterchainTokenService Admin to Emergency Operator EOA - refer to the Target Role Addresses Table above

```bash
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>
ITS_CONTRACT=$(jq -r '.axelar.contracts.InterchainTokenService.address' ./axelar-chains-config/info/$ENV.json)

# Execute admin update
axelard tx wasm execute $ITS_CONTRACT \
  '{"update_admin": {"admin": "'$EMERGENCY_OPERATOR_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# Verify transfer
axelard query wasm contract-state smart $ITS_CONTRACT \
  '{"adminAddress": {}}'
```

### Step 7: Set InterchainTokenService Operator to Relayer Operators EOA - refer to the Target Role Addresses Table above

```bash
RELAYER_OPERATORS_EOA=<RELAYER_OPERATORS_EOA_ADDRESS>
ITS_CONTRACT=$(jq -r '.axelar.contracts.InterchainTokenService.address' ./axelar-chains-config/info/$ENV.json)

# Set operator (this operation might be named differently based on contract implementation)
axelard tx wasm execute $ITS_CONTRACT \
  '{"set_operator": {"operator": "'$RELAYER_OPERATORS_EOA'"}}' \
  --from <current_admin_or_governance_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# Verify operator
axelard query wasm contract-state smart $ITS_CONTRACT \
  '{"operator": {}}'
```

### Step 8: Transfer XRPL Contract Admin Roles to Emergency Operator EOA

Transfer admin roles for XrplVotingVerifier, XrplGateway, and XrplMultisigProver to Emergency Operator EOA - refer to the Target Role Addresses Table above.


```bash
EMERGENCY_OPERATOR_EOA=<EMERGENCY_OPERATOR_EOA_ADDRESS>

# XrplVotingVerifier
XRPL_VOTING_VERIFIER=$(jq -r '.axelar.contracts.XrplVotingVerifier.address' ./axelar-chains-config/info/$ENV.json)
axelard tx wasm execute $XRPL_VOTING_VERIFIER \
  '{"update_admin": {"admin": "'$EMERGENCY_OPERATOR_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# XrplGateway
XRPL_GATEWAY=$(jq -r '.axelar.contracts.XrplGateway.address' ./axelar-chains-config/info/$ENV.json)
axelard tx wasm execute $XRPL_GATEWAY \
  '{"update_admin": {"admin": "'$EMERGENCY_OPERATOR_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# XrplMultisigProver
XRPL_MULTISIG_PROVER=$(jq -r '.axelar.contracts.XrplMultisigProver.address' ./axelar-chains-config/info/$ENV.json)
axelard tx wasm execute $XRPL_MULTISIG_PROVER \
  '{"update_admin": {"admin": "'$EMERGENCY_OPERATOR_EOA'"}}' \
  --from <current_admin_key> \
  --chain-id <chain_id> \
  --gas auto \
  --gas-adjustment 1.5 \
  --fees 5000uaxl

# Verify all transfers
axelard query wasm contract-state smart $XRPL_VOTING_VERIFIER '{"adminAddress": {}}'
axelard query wasm contract-state smart $XRPL_GATEWAY '{"adminAddress": {}}'
axelard query wasm contract-state smart $XRPL_MULTISIG_PROVER '{"adminAddress": {}}'
```

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

