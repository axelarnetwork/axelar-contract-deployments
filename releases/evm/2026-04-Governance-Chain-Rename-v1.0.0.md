## Governance Chain Rename (Axelarnet → axelar) v1.0.0

|                | **Owner**                               |
| -------------- | --------------------------------------- |
| **Created By** | @rista404 <ristic@commonprefix.com>       |
| **Deployment** |                                         |

| **Environment**      | **Chain**           | **Deployment Status** | **Date** |
| -------------------- | ------------------- | --------------------- | -------- |
| **Devnet Amplifier** | `core-avalanche`    | Completed             | 23/04/2026 |
|                      | `core-ethereum`     | Completed             | 23/04/2026 |
|                      | `core-optimism`     | Completed             | 23/04/2026 |
| **Stagenet**         | `avalanche`         | Completed             | 04/05/2026 |
|                      | `fantom`            | Skipped               | 04/05/2026 |
|                      | `kava`              | Completed             | 04/05/2026 |
|                      | `ethereum-sepolia`  | Completed             | 04/05/2026 |
|                      | `arbitrum-sepolia`  | Completed             | 04/05/2026 |
|                      | `linea-sepolia`     | Completed             | 06/05/2026 |
|                      | `polygon-sepolia`   | Completed             | 06/05/2026 |
|                      | `base-sepolia`      | Completed             | 06/05/2026 |
|                      | `mantle-sepolia`    | Completed             | 06/05/2026 |
|                      | `optimism-sepolia`  | Completed             | 06/05/2026 |
| **Testnet**          | `ethereum-sepolia`  | Completed             | 13/05/2026 |
|                      | `avalanche`         | Completed             | 13/05/2026 |
|                      | `moonbeam`          | Completed             | 13/05/2026 |
|                      | `binance`           | Completed             | 13/05/2026 |
|                      | `kava`              | Completed             | 13/05/2026 |
|                      | `filecoin-2`        | Completed             | 13/05/2026 |
|                      | `scroll`            | Completed             | 13/05/2026 |
|                      | `immutable`         | Completed             | 13/05/2026 |
|                      | `arbitrum-sepolia`  | Completed             | 13/05/2026 |
|                      | `optimism-sepolia`  | Completed             | 13/05/2026 |
|                      | `base-sepolia`      | Completed             | 13/05/2026 |
|                      | `blast-sepolia`     | Skipped               | 13/05/2026 |
|                      | `mantle-sepolia`    | Completed             | 13/05/2026 |
|                      | `polygon-sepolia`   | Completed             | 13/05/2026 |
|                      | `linea-sepolia`     | Completed             | 13/05/2026 |
| **Mainnet**          | `celo`              | Completed             | 15/06/2026 |
|                      | `ethereum`          | Completed             | 15/06/2026 |
|                      | `avalanche`         | Completed             | 15/06/2026 |
|                      | `polygon`           | Completed             | 15/06/2026 |
|                      | `moonbeam`          | Completed             | 15/06/2026 |
|                      | `binance`           | Completed             | 15/06/2026 |
|                      | `arbitrum`          | Completed             | 15/06/2026 |
|                      | `kava`              | Completed             | 15/06/2026 |
|                      | `filecoin`          | Completed             | 15/06/2026 |
|                      | `optimism`          | Completed             | 15/06/2026 |
|                      | `linea`             | Completed             | 15/06/2026 |
|                      | `base`              | Completed             | 15/06/2026 |
|                      | `mantle`            | Completed             | 15/06/2026 |
|                      | `scroll`            | Completed             | 15/06/2026 |
|                      | `immutable`         | Completed             | 15/06/2026 |
|                      | `fraxtal`           | Completed             | 15/06/2026 |
|                      | `blast`             | Completed             | 15/06/2026 |

## Background

The legacy consensus EVM chains have `InterchainGovernance`/`AxelarServiceGovernance` contracts deployed with `governanceChain = "Axelarnet"`. The canonical chain name is now `"axelar"`. Because `governanceChain` is only assignable in the constructor and neither contract is upgradeable, each affected governance contract must be **redeployed** with `governanceChain = "axelar"`, and all roles currently held by the old governance contract must be **transferred** to the new one.

### Role Migration Summary

Only roles currently held by the legacy `"Axelarnet"` governance contract are in scope. Roles held by EOAs or separately-owned operator addresses are out of scope (handled by the 2025-11 Consensus Role Transfers release).

| Contract                | Role       | Current Holder                      | Assign To                     | Call                                            |
| ----------------------- | ---------- | ----------------------------------- | ----------------------------- | ----------------------------------------------- |
| AxelarGateway           | governance | Legacy `*Governance` (Axelarnet)    | New `AxelarServiceGovernance` | `transferGovernance(newGovernance)`             |
| AxelarGasService        | owner      | Legacy `*Governance` (Axelarnet)    | New `AxelarServiceGovernance` | `transferOwnership(newOwner)`                   |
| InterchainTokenService  | owner      | Legacy `*Governance` (Axelarnet)    | New `AxelarServiceGovernance` | `transferOwnership(newOwner)`                   |

Per-chain scope is conditional: a role is migrated only when on-chain `governance()`/`owner()` currently returns the legacy governance contract. See per-environment tables below.

## Pre-requisites

| Network              | Chains                                                                                                                                                                            |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Devnet Amplifier** | `core-avalanche,core-ethereum,core-optimism`                                                                                                                                      |
| **Stagenet**         | `avalanche,kava,ethereum-sepolia,arbitrum-sepolia,linea-sepolia,polygon-sepolia,base-sepolia,mantle-sepolia,optimism-sepolia` (fantom skipped — see notes) |
| **Testnet**          | `ethereum-sepolia,avalanche,moonbeam,binance,kava,filecoin-2,scroll,immutable,arbitrum-sepolia,optimism-sepolia,base-sepolia,mantle-sepolia,polygon-sepolia,linea-sepolia` (blast-sepolia skipped — see notes) |
| **Mainnet**          | `celo,ethereum,avalanche,polygon,moonbeam,binance,arbitrum,kava,filecoin,optimism,linea,base,mantle,scroll,immutable,fraxtal,blast` (fantom + centrifuge skipped — see notes) |

1. Update npm dependencies:

```bash
npm ci && npm run build
```

2. Create an `.env` config:

```yaml
PRIVATE_KEY=<deployer private key>
ENV=<devnet-amplifier|stagenet|testnet|mainnet>
CHAIN=<chain name>
```

## Deployment Steps

### Step 1: Deploy new `AxelarServiceGovernance` with `governanceChain = "axelar"`

For each chain in the per-env list, update `axelar-chains-config/info/${ENV}.json`:

- Set `AxelarServiceGovernance.governanceChain` to `"axelar"` (or omit — the deployer defaults to `"axelar"`).
- Bump the `AxelarServiceGovernance.salt` so CREATE2/CREATE3 produces a new address (e.g. `v6.0.5`).
- Keep `governanceAddress`, `minimumTimeDelay`, `operator` as in the existing config.

Deploy:

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance -m create2 --parallel
```

`deploy-contract.js` verifies `governanceChain`, `governanceChainHash`, `governanceAddress`, `governanceAddressHash`, `minimumTimeDelay`, and `operator` on-chain against the config after deploy.

#### Devnet-Amplifier (completed 2026-04-23)

| Chain            | New AxelarServiceGovernance                  |
| ---------------- | -------------------------------------------- |
| `core-avalanche` | `0xC6BF5eDa5dB1DeD52c38eEbB1Bec27e4BEcc757a` |
| `core-ethereum`  | `0xe7E7fD6BF45329b2c63cd43d541C7ab12f177140` |
| `core-optimism`  | `0x08f116A026285073987eeFF68eFDbA4E36231a91` |

Salt used: `v6.0.4-axelar devnet-amplifier` (CREATE2).

#### Stagenet (completed 2026-05-06)

| Chain               | New AxelarServiceGovernance                  | Completed   |
| ------------------- | -------------------------------------------- | ----------- |
| `avalanche`         | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-04  |
| `kava`              | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-04  |
| `ethereum-sepolia`  | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-04  |
| `arbitrum-sepolia`  | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-04  |
| `linea-sepolia`     | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-06  |
| `polygon-sepolia`   | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-06  |
| `base-sepolia`      | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-06  |
| `mantle-sepolia`    | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-06  |
| `optimism-sepolia`  | `0x3239fAe62FDF4A2E14779a335A19598d58b16B9B` | 2026-05-06  |

Salt used: `v6.0.4-axelar` (CREATE2). Same deployer + salt + bytecode across all chains, hence the same deployed address. Roles transferred (gateway governance, gas service owner, ITS owner) all verified on-chain per chain.

#### Testnet (completed 2026-05-13)

| Chain               | New AxelarServiceGovernance                  | Completed   |
| ------------------- | -------------------------------------------- | ----------- |
| `ethereum-sepolia`  | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `avalanche`         | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `moonbeam`          | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `binance`           | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `kava`              | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `filecoin-2`        | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `scroll`            | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `immutable`         | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `arbitrum-sepolia`  | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `optimism-sepolia`  | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `base-sepolia`      | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `mantle-sepolia`    | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `polygon-sepolia`   | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |
| `linea-sepolia`     | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` | 2026-05-13  |

Salt used: `v6.0.4-axelar` (CREATE3). Same deployer + salt across all chains → identical address everywhere. Operator EOA: `0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e`. All gateway governance transfers verified on-chain.

The Step 2 transfers were submitted as a single batched Axelar proposal (id `568`) carrying 14 `contract_calls[]` entries, one deposit of 1M AXL (refunded on pass) covering the per-call min-deposit gate on the `axelarnet` module.

**`blast-sepolia` skipped**: both the RPC endpoint (`sepolia.blast.io`, load-balanced) and the underlying chain were unstable during the migration window — the deploy tx repeatedly failed on a "nonce too low" mismatch between the RPC's view and the chain's accepted nonce, and block production was intermittent. Revisit when the network stabilizes.

### Step 2: Transfer roles held by the legacy governance contract

For each row in the per-env tables below, submit an Axelar governance proposal that routes through the **legacy governance contract** (column `Legacy governance`) and calls `transferGovernance` / `transferOwnership` on the target contract, handing the role to the new `AxelarServiceGovernance` (from Step 1).

Two inner calldatas are used in this release:

```bash
# Gateway role transfer
cast calldata "transferGovernance(address)" <new-governance>

# Gas service / ITS / Operators (Ownable) role transfer
cast calldata "transferOwnership(address)"  <new-governance>
```

Per-invocation command:

```bash
ts-node evm/governance.js schedule raw <activationTime> \
  --address <legacy-governance> \
  --target <target> \
  --calldata <calldata> \
  -n <chain>
```

Then, once the timelock elapses (`minimumTimeDelay` per env), anyone can finalize by running `evm/governance.js execute` with the same `--address`, `--target`, `--calldata`, `-n` arguments.

`--address` pins the legacy governance contract explicitly and bypasses the JSON lookup (which we've overwritten with the new address in Step 1). `--target` is the contract being reconfigured (gateway / gas service / ITS). `--calldata` is the `transferGovernance(newGov)` or `transferOwnership(newGov)` payload.

`<activationTime>` is a UTC `YYYY-MM-DDTHH:mm:ss` or `0` (= earliest allowed by `minimumTimeDelay`).

#### Devnet-Amplifier

| Chain            | Gateway (target)                             | Legacy governance (`--address`)              | New governance                               | Calldata                                                                     |
| ---------------- | -------------------------------------------- | -------------------------------------------- | -------------------------------------------- | ---------------------------------------------------------------------------- |
| `core-avalanche` | `0xb7879887ec7e85a5C757D7ccF5E3AB15007152e2` | `0xfB71a4d90c37C9BCE7cD4Cb692cE26EA3AC0A319` | `0xC6BF5eDa5dB1DeD52c38eEbB1Bec27e4BEcc757a` | `0xd38bfff4000000000000000000000000c6bf5eda5db1ded52c38eebb1bec27e4becc757a` |
| `core-ethereum`  | `0x7C60aA56482c2e78D75Fd6B380e1AdC537B97319` | `0x677c130e0f17F91F7361AcC15b8d7e6A3D6ECeeb` | `0xe7E7fD6BF45329b2c63cd43d541C7ab12f177140` | `0xd38bfff4000000000000000000000000e7e7fd6bf45329b2c63cd43d541c7ab12f177140` |
| `core-optimism`  | `0xdb6711cFc97e13E4aF6EEEe5f4923A9c2FBf0721` | `0xF37E8D9FaB4FF223BC3E1Fe4EA6aA2fd6D95F2eb` | `0x08f116A026285073987eeFF68eFDbA4E36231a91` | `0xd38bfff400000000000000000000000008f116a026285073987eeff68efdba4e36231a91` |

Use `activationTime = 0` (devnet `minimumTimeDelay = 0`).

#### Stagenet

All chains share the legacy governance contract `0x7B1cfbC6F267494f1A187C134E14A2B34CC3C550`. For each chain, submit 3 proposals: gateway `transferGovernance`, gas service `transferOwnership`, ITS `transferOwnership`. All point to the same new governance per-chain (from Step 1).

Chains: `avalanche, kava, ethereum-sepolia, arbitrum-sepolia, linea-sepolia, polygon-sepolia, base-sepolia, mantle-sepolia, optimism-sepolia`.

`fantom` is **skipped** on stagenet: Fantom testnet (chain 4002) was abandoned in the Sonic rebrand and the RPC is unreachable. The contracts can no longer be migrated. Note: Fantom **mainnet** (chain 250) is still operational and remains in scope for the mainnet rollout.

Use `activationTime = 0` (stagenet `minimumTimeDelay = 300s`; proposal becomes executable 300s after scheduling).

Targets per chain (queryable post-Step-1):
- Gateway: `.chains[<chain>].contracts.AxelarGateway.address`
- Gas service: `.chains[<chain>].contracts.AxelarGasService.address`
- ITS: `.chains[<chain>].contracts.InterchainTokenService.address`
- New governance (to pass into calldata): `.chains[<chain>].contracts.AxelarServiceGovernance.address`

#### Testnet

All chains share the legacy governance contract `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`. Only the gateway is migrated; gas service / operators / ITS are EOA-owned on testnet.

Chains: `ethereum-sepolia, avalanche, moonbeam, binance, kava, filecoin-2, scroll, immutable, arbitrum-sepolia, optimism-sepolia, base-sepolia, mantle-sepolia, polygon-sepolia, linea-sepolia`.

`blast-sepolia` skipped — RPC and chain were unstable during the migration window. See note under Step 1.

Use `activationTime = 0` (testnet `minimumTimeDelay = 300s`).

#### Mainnet

16 chains share `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`. `immutable` has its own legacy governance `0x35dFacdE7B4b80e156e69b1291D12EA51ce123BD`. Only the gateway is migrated; other roles are EOA-owned.

Chains (shared gov): `celo, ethereum, avalanche, polygon, moonbeam, binance, arbitrum, kava, filecoin, optimism, linea, base, mantle, scroll, fraxtal, blast`.
Separate gov: `immutable`.

`fantom` and `centrifuge` skipped — out of scope, both are being deprecated.

Use `activationTime = 0`. The on-chain `minimumTimeLockDelay` on each legacy `InterchainGovernance` is `604800s / 7 days` — that's the actual wait gating Step 2 execute. (The new `AxelarServiceGovernance` we deploy in Step 1 has its own `minimumTimeDelay = 259200s / 72h`, which applies to *future* proposals routed through the new contract, not to this migration.)

#### Mainnet (completed 2026-06-15)

New `AxelarServiceGovernance` deployed via CREATE3 at the same address across all 17 chains:

- **Address**: `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`
- **Salt**: `v6.0.4-axelar`
- **Deployer**: `0xba76c6980428A0b10CFC5d8ccb61949677A61233`
- **Operator**: `0xc8a8399c3D1207f4e109673be7047604737c1D56`
- **minimumTimeDelay**: 259200 (72h)
- **governanceChain**: `axelar`

Step 2 was submitted as one batched Axelar `CallContractsProposal` (proposal id `474`) carrying 17 `contract_calls[]` entries (16 routed to `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`, one to `0x35dFacdE7B4b80e156e69b1291D12EA51ce123BD` for `immutable`), signed by the `cpi-multisig` (3/6, `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m`) with a 1,000,000 AXL deposit. Voting ran the standard ~3 day period; the 7-day per-chain `InterchainGovernance` timelock followed; execute was permissionless, run via `helpers/mainnet-migrate.sh`. All 17 gateways verified post-execute: `gateway.governance() == 0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`.

### Step 3: Update config JSON

For each migrated chain, update `axelar-chains-config/info/${ENV}.json`:

- Remove the legacy `InterchainGovernance` / old `AxelarServiceGovernance` entry if still present.
- Confirm the new `AxelarServiceGovernance` entry reflects the new address and `governanceChain = "axelar"`.

## Checklist

Per chain:

- [ ] New `AxelarServiceGovernance` deployed with `governanceChain = "axelar"` (verified on-chain).
- [ ] `AxelarGateway.governance()` returns the new `AxelarServiceGovernance`.
- [ ] Stagenet only: `AxelarGasService.owner()` returns the new `AxelarServiceGovernance`.
- [ ] Stagenet only: `InterchainTokenService.owner()` returns the new `AxelarServiceGovernance`.
- [ ] Legacy governance contract no longer holds any tracked role on-chain.
- [ ] `${ENV}.json` reflects the new addresses.
