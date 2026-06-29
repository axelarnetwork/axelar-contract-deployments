## InterchainTokenService Ownership Transfer to Governance

|                | **Owner**                            |
| -------------- | ------------------------------------ |
| **Created By** | @rista404 <ristic@commonprefix.com>  |
| **Deployment** |                                      |

| **Environment** | **Chain**     | **Deployment Status**        | **Date**   |
| --------------- | ------------- | ---------------------------- | ---------- |
| **Mainnet**     | `xrpl-evm`    | Step 1 Completed             | 12/06/2026 |
|                 | `hedera`      | Step 1 Completed             | 12/06/2026 |
|                 | `hyperliquid` | Step 1 Completed             | 12/06/2026 |
|                 | `monad`       | Step 1 Completed             | 12/06/2026 |
|                 | `celo`        | Step 1 Skipped (ASG present) | -          |
|                 | `ethereum`    | Step 1 Skipped (ASG present) | -          |
|                 | `avalanche`   | Step 1 Skipped (ASG present) | -          |
|                 | `polygon`     | Step 1 Skipped (ASG present) | -          |
|                 | `moonbeam`    | Step 1 Skipped (ASG present) | -          |
|                 | `binance`     | Step 1 Skipped (ASG present) | -          |
|                 | `arbitrum`    | Step 1 Skipped (ASG present) | -          |
|                 | `kava`        | Step 1 Skipped (ASG present) | -          |
|                 | `filecoin`    | Step 1 Skipped (ASG present) | -          |
|                 | `optimism`    | Step 1 Skipped (ASG present) | -          |
|                 | `linea`       | Step 1 Skipped (ASG present) | -          |
|                 | `base`        | Step 1 Skipped (ASG present) | -          |
|                 | `mantle`      | Step 1 Skipped (ASG present) | -          |
|                 | `scroll`      | Step 1 Skipped (ASG present) | -          |
|                 | `immutable`   | Step 1 Skipped (ASG present) | -          |
|                 | `fraxtal`     | Step 1 Skipped (ASG present) | -          |
|                 | `blast`       | Step 1 Skipped (ASG present) | -          |

## Background

This release brings `InterchainTokenService` (ITS) ownership across mainnet under
`AxelarServiceGovernance` (ASG). Today the ITS `Ownable` owner on each chain is an
externally-managed address rather than the governance contract; this release transfers that
ownership to the ASG.

`AxelarServiceGovernance` is already deployed at the same CREATE3 address
(`0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`) on most mainnet chains. A handful of chains
(`xrpl-evm`, `hedera`, `hyperliquid`, `monad`) were brought up without any on-chain
governance contract, so there is nothing to transfer ITS ownership *to* there yet. The rollout
is therefore two stages:

- **Step 1 — deploy ASG where it is missing.** Only the 4 chains above need this; every other
  chain already has the ASG and **skips Step 1**.
- **Step 2 — transfer the ITS owner to the ASG.** Runs on all in-scope chains.

In scope for this release:

| Contract               | Role  | Current Holder        | Assign To                     | Call                          |
| ---------------------- | ----- | --------------------- | ----------------------------- | ----------------------------- |
| InterchainTokenService | owner | see per-chain table   | `AxelarServiceGovernance`     | `transferOwnership(newOwner)` |

`AxelarGateway` and `AxelarGasService` ownership transfers follow the same pattern and are
tracked separately — they are **not** part of this doc.

### A note on the salt vs. the contract version

The ASG is deployed with salt **`v6.0.4-axelar`**. This salt string does **not** necessarily
correspond to the contract's source version — it is reused deliberately. CREATE3 addresses
derive from `(create3 factory, deployer EOA, salt)` only, independent of the contract bytecode
or version. By reusing the same salt (and the same deployer `0xba76c6980428A0b10CFC5d8ccb61949677A61233`
and the same factory `0x6513Aedb4D1593BA12e50644401D976aebDc90d8`, which is present on every
target chain), the ASG lands at the **same address on all chains** —
`0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`. Read the salt as an "address-coordination tag,"
not a version number.

## Pre-requisites

| Network     | Chains (Step 1 — deploy ASG)             |
| ----------- | ---------------------------------------- |
| **Mainnet** | `xrpl-evm,hedera,hyperliquid,monad` |

1. Update npm dependencies:

```bash
npm ci && npm run build
```

2. Create an `.env` config. **Step 1 must be deployed from the canonical ASG deployer**
   (`0xba76c6980428A0b10CFC5d8ccb61949677A61233`) so the CREATE3 address matches the other chains:

```yaml
PRIVATE_KEY=<deployer private key for 0xba76c6980428A0b10CFC5d8ccb61949677A61233>
ENV=mainnet
```

## Deployment Steps

### Step 1: Deploy `AxelarServiceGovernance` where missing (Completed 2026-06-12)

> Applies only to `xrpl-evm,hedera,hyperliquid,monad`. **Skip for every other chain** —
> the ASG is already deployed there at `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`.

The config for each chain (`axelar-chains-config/info/mainnet.json`) carries:

- `governanceChain = "axelar"`
- `governanceAddress = "axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj"`
- `minimumTimeDelay = 259200` (72h)
- `operator = "0xc8a8399c3D1207f4e109673be7047604737c1D56"`
- `deploymentMethod = "create3"`, `salt = "v6.0.4-axelar"`

Predict first (every chain must print `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`):

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance -m create3 \
  -n xrpl-evm,hedera,hyperliquid,monad --predictOnly
```

Deploy:

```bash
ts-node evm/deploy-contract.js -c AxelarServiceGovernance -m create3 \
  -n xrpl-evm,hedera,hyperliquid,monad --parallel
```

`deploy-contract.js` verifies `governanceChain`, `governanceChainHash`, `governanceAddress`,
`governanceAddressHash`, `minimumTimeDelay`, and `operator` on-chain against the config after deploy.

New `AxelarServiceGovernance` deployed via CREATE3 at the same address on all 4 chains:

| Chain         | AxelarServiceGovernance                      |
| ------------- | -------------------------------------------- |
| `xrpl-evm`    | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` |
| `hedera`      | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` |
| `hyperliquid` | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` |
| `monad`       | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` |

- **Salt**: `v6.0.4-axelar` (CREATE3) — see "salt vs. version" note above.
- **Deployer**: `0xba76c6980428A0b10CFC5d8ccb61949677A61233`
- **Operator**: `0xc8a8399c3D1207f4e109673be7047604737c1D56`
- **minimumTimeDelay**: 259200 (72h)
- **governanceChain**: `axelar`

### Step 2: Transfer `InterchainTokenService` ownership to the ASG

Each transfer is initiated by the **current ITS owner** of that chain (see table) and calls
`transferOwnership(0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525)` on the ITS proxy
`0xB5FB4BE02232B1bBA4dC8f81dc24C26980dE9e3C`. Execute it through whichever mechanism controls
that owner address. `Ownable.transferOwnership` is single-step: ITS ownership moves to the ASG
immediately, with no separate accept.

Inner calldata (same for every chain):

```bash
cast calldata "transferOwnership(address)" 0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525
# 0xf2fde38b0000000000000000000000007acbae6cba67d78aaf69e47000884ae00f9b2525
```

| Chain         | Current ITS owner                            | Step 1 (ASG)   |
| ------------- | -------------------------------------------- | -------------- |
| `xrpl-evm`    | `0x3DEbBA1a03799EE1e30aA82C2cd90CF395643978` | deployed       |
| `hedera`      | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | deployed       |
| `hyperliquid` | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | deployed       |
| `monad`       | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | deployed       |
| `celo`        | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `ethereum`    | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `avalanche`   | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `polygon`     | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `moonbeam`    | `0x965127d8b0e64d053a24e28A42fda71edE5101fd` | skipped        |
| `binance`     | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `arbitrum`    | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `kava`        | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | skipped        |
| `filecoin`    | `0x965127d8b0e64d053a24e28A42fda71edE5101fd` | skipped        |
| `optimism`    | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `linea`       | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `base`        | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `mantle`      | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `scroll`      | `0x5f939a751eAeE302C85bf8BeBB83483aDeCC0C10` | skipped        |
| `immutable`   | `0x965127d8b0e64d053a24e28A42fda71edE5101fd` | skipped        |
| `fraxtal`     | `0x41b2b0cb19F74aEc556472CCb965bd04bD869B03` | skipped        |
| `blast`       | `0x6f24A47Fc8AE5441Eb47EFfC3665e70e69Ac3F05` | skipped        |

### Step 3: Update config JSON

After each transfer, confirm `axelar-chains-config/info/mainnet.json` reflects the deployed
ASG address (written automatically by Step 1) and record the completed ownership transfer.

## Checklist

Per chain:

- [ ] `AxelarServiceGovernance` present at `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` with `governanceChain = "axelar"` (verified on-chain).
- [ ] `InterchainTokenService.owner()` returns the `AxelarServiceGovernance`.
- [ ] Previous owner no longer holds the ITS owner role.
- [ ] `mainnet.json` reflects the ASG address.
