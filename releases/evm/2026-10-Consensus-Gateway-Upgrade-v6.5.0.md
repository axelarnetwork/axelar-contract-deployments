# Consensus Gateway Upgrade + Auth Module Retrofit v6.5.0

|                | **Owner**                           |
| -------------- | ----------------------------------- |
| **Created By** | @jakovmitrovski, @h4writer, @dolfje |
| **Deployment** | @h4writer <hannes@commonprefix.com> |

| **Environment**      | **Chain**                           | **Deployment Status** | **Date**   |
| -------------------- | ----------------------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | `avalanche`, `ethereum`, `optimism` | Completed             | 2026-09-01 |
| **Stagenet**         | all 9 active consensus chains       | Completed             | 2026-09-02 |
| **Testnet**          | `linea-sepolia` (pilot)             | -                     | TBD        |
|                      | `ethereum-sepolia`                  | -                     | TBD        |
|                      | `avalanche`                         | -                     | TBD        |
|                      | `moonbeam`                          | -                     | TBD        |
|                      | `binance`                           | -                     | TBD        |
|                      | `kava`                              | -                     | TBD        |
|                      | `filecoin-2`                        | -                     | TBD        |
|                      | `scroll`                            | -                     | TBD        |
|                      | `immutable`                         | -                     | TBD        |
|                      | `arbitrum-sepolia`                  | -                     | TBD        |
|                      | `optimism-sepolia`                  | -                     | TBD        |
|                      | `base-sepolia`                      | -                     | TBD        |
|                      | `mantle-sepolia`                    | -                     | TBD        |
|                      | `polygon-sepolia`                   | -                     | TBD        |
| **Mainnet**          | -                                   | -                     | TBD        |

- [Runbook](https://app.notion.com/p/commonprefix/Consensus-Gateway-Upgrade-3cdbf672de8281529199d4cf4f065d4a)
- [axelar-cgp-solidity v6.5.0](https://github.com/axelarnetwork/axelar-cgp-solidity/releases/tag/v6.5.0)

## Background

Upgrade the consensus (legacy) `AxelarGateway` implementation to cgp-solidity `v6.5.0`, which binds to a new
`AxelarAuthWeighted`. `authModule` is a constructor `immutable` on the implementation, so the upgrade needs a
replacement auth module. The retrofit works like this:

1. Deploy the auth **unseeded** at a CREATE2 address, owned by the deployer.
2. Deploy the implementation bound to that auth.
3. Schedule the upgrade through `AxelarServiceGovernance` (ASG) and get it through Axelar governance.
4. Seed the auth with the recent operator sets **just before** execution.
5. Execute the upgrade.
6. Hand auth ownership to the gateway proxy **right after**.
7. Set the pauser. 6.5.0 adds a `pauser` role that can pause the gateway without the timelock. `setup()` does not set
   it, so after the upgrade `pauser()` is `0x0` until governance calls `transferPauser`.

Why the timing matters: `validateProof` only accepts operator sets within `OLD_KEY_RETENTION = 16` epochs of the
newest one, and operators rotate about once a day. If you seed too early, the signing set ages out before execution.
Between execute and handoff the seeder owns the auth, so new rotations do not register in it. After 16 rotations
(about 16 days) every batch reverts, and `transferOperatorship` cannot fix that because it arrives inside a batch.

### Scope

| Network     | Chains                                                                                                                                                                                               |
| ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Testnet** | `linea-sepolia` first, as a pilot. Then `ethereum-sepolia,avalanche,moonbeam,binance,kava,filecoin-2,scroll,immutable,arbitrum-sepolia,optimism-sepolia,base-sepolia,mantle-sepolia,polygon-sepolia` |

Out of scope on testnet:

- `blast-sepolia`, `centrifuge-2`: on-chain `governance()` is `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`, not the ASG.
- `fantom`: RPC is dead.

Before including a chain, check on-chain that `governance()` returns the ASG (`0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`).
The config `governance` field is stale for `moonbeam` and `scroll`.

### Pilot: linea-sepolia

| Item                    | Value                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------ |
| Gateway proxy           | `0xe432150cce91c13a887f7D836923d5597adD8E31`                                         |
| Current implementation  | `0xc1712652326E87D193Ac11910934085FF45C2F48`                                         |
| Current auth module     | `0x1a920B29eBD437074225cAeE44f78FC700B27a5d`                                         |
| AxelarServiceGovernance | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` (`minimumTimeDelay` 300)                |
| ASG operator            | `0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e`                                         |
| Deployer / auth seeder  | `0xBF6d81aDCfCa1D7Bea60E23C1da5483F84E4A668` ([Testnet] Deployer EOA: EVM)           |
| Pauser (new role)       | `0xF140b31cBe9F6a3b1d52491b84bABEDAa2962faB` ([Testnet] Emergency Operator EOA: EVM) |
| Explorer                | https://sepolia.lineascan.build/                                                     |

The salts below give an auth and an implementation address that the rest of the testnet batch can share
(same deployer, same salt). Do not reuse them for a different auth or implementation version.

| Contract             | Method    | Salt                                | Predicted address (deployer `0xBF6d…`)       |
| -------------------- | --------- | ----------------------------------- | -------------------------------------------- |
| `AxelarAuthWeighted` | `create2` | `AxelarAuthWeighted v6.5.0 testnet` | `0x96e4D329fd6099b40446d660b3A72CA9aCC94EC6` |
| `AxelarGateway` impl | `create3` | `AxelarGateway v6.5`                | `0x53A68B407392F8A39cc6418c939f4d9cb75350D7` |

Earlier environments: both were deployed by `0xba76c6980428A0b10CFC5d8ccb61949677A61233`. The auth salt was
`AxelarAuthWeighted v6.5.0` on devnet-amplifier (`0x7d940759…`) and `AxelarAuthWeighted v6.5.0 stagenet` on stagenet
(`0xb929e41b…`). Both deployed the implementation with plain `create`, so it has a different address on every chain.

The implementation uses `create3`, as the current mainnet and testnet `v6.2` implementations did (salt `AxelarGateway v6.2`).
A `create3` address depends only on the deployer and the salt, not on the bytecode, so every chain gets the same implementation
address even though each one binds a different auth. The deployer decides the address: `0x6f24…` gives `0x99B5FA03…` and
`0xB8Cd…` gives `0xc1712652…`, on both mainnet and testnet. Each chain records its own `implementationCodehash`.

## Pre-requisites

1. Update npm dependencies

```bash
npm ci && npm run build
```

2. Set the shell variables (fish). `PRIVATE_KEY` is the deployer `0xBF6d…`, and `MNEMONIC` is the Axelar wallet that
   submits the governance proposals. The scripts read both from the environment. An exported variable takes precedence
   over `.env`, but if you forget to export it, a `PRIVATE_KEY` left in `.env` is used silently.

```fish
read -sx PRIVATE_KEY
read -sx MNEMONIC
set -x ENV testnet
set -x CHAIN linea-sepolia

set GATEWAY 0xe432150cce91c13a887f7D836923d5597adD8E31
set AUTH_SALT "AxelarAuthWeighted v6.5.0 testnet"
set IMPL_SALT "AxelarGateway v6.5"
set AUTH 0x96e4D329fd6099b40446d660b3A72CA9aCC94EC6 # predicted, confirmed in Step 1
set IMPL 0x53A68B407392F8A39cc6418c939f4d9cb75350D7 # predicted, confirmed in Step 2
set PAUSER 0xF140b31cBe9F6a3b1d52491b84bABEDAa2962faB
set PAUSER_CALLDATA 0x4421ea21000000000000000000000000f140b31cbe9f6a3b1d52491b84babedaa2962fab # transferPauser($PAUSER)
```

3. Check that the deployer is funded on `$CHAIN` and that the gateway is governed by the ASG:

```fish
cast call $GATEWAY "governance()(address)" --rpc-url https://rpc.sepolia.linea.build
```

## Deployment Steps

### Step 1: Deploy the unseeded auth module

```fish
ts-node evm/consensus-auth.js predict -e $ENV -n $CHAIN -s $AUTH_SALT
ts-node evm/consensus-auth.js deploy  -e $ENV -n $CHAIN -s $AUTH_SALT
```

Check that the printed auth address equals `$AUTH`. It stays unseeded and owned by the deployer until Steps 4 and 6.

### Step 2: Deploy the implementation bound to `$AUTH`

```fish
ts-node evm/deploy-consensus-gateway.js -e $ENV -n $CHAIN \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT
```

- Ownership transfer to the proxy is skipped, and config records `$AUTH` as `AxelarGateway.pendingAuthModule`.
- Check that the printed implementation address equals `$IMPL`.
- Do **not** pass `--upgrade`. The gateway is ASG-governed, so the upgrade has to go through governance.

### Step 3: Schedule the upgrade through governance

Submit as an **expedited** proposal, which is the `governance.js` default. On testnet that means a 300s voting period,
a 66.7% threshold and a 3,000 AXL deposit per proposal (6,000 AXL for Steps 3 and 3b, refunded unless vetoed).
The activation time must cover the vote, the relay and the 300s ASG `minimumTimeDelay`, so about 30 minutes ahead is a
safe choice. It is a UTC timestamp (`YYYY-MM-DDTHH:mm:ss`).

```fish
set ACTIVATION_TIME (date -u -d '+30 min' +%Y-%m-%dT%H:%M:%S)

ts-node evm/governance.js schedule upgrade $ACTIVATION_TIME -e $ENV -n $CHAIN \
  --targetContractName AxelarGateway --implementation $IMPL \
  --title "Upgrade $CHAIN consensus gateway to v6.5.0" \
  --description "Upgrades the $CHAIN AxelarGateway to $IMPL, bound to the new AxelarAuthWeighted $AUTH"
```

- `setupParams` is `0x`. Do not pass `--newGovernance` / `--newMintLimiter`.
- The printed target must equal `$GATEWAY`. Save the printed calldata for Step 5:

```fish
set UPGRADE_CALLDATA <printed calldata>
```

- With `MNEMONIC` set, the proposal is submitted directly. To review it first, use `--generate-only proposal.json`.

### Step 3b: Schedule the pauser transfer through governance

Submit this right after Step 3 so both proposals go through voting together. `transferPauser` only exists on the
new implementation, so it must be **executed after** the upgrade (Step 5b).

```fish
ts-node evm/governance.js schedule raw $ACTIVATION_TIME -e $ENV -n $CHAIN \
  --target $GATEWAY --calldata $PAUSER_CALLDATA \
  --title "Set $CHAIN consensus gateway pauser" \
  --description "Calls transferPauser($PAUSER) on the $CHAIN AxelarGateway"
```

Wait for both proposals to pass and for both timelocks to be scheduled on the ASG.

### Step 4: Seed the auth and verify

Run this right before execution, once the proposal has relayed. If the ETA slips by more than a few rotations, re-run `seed`.

```fish
ts-node evm/consensus-auth.js seed   -e $ENV -n $CHAIN --authModule $AUTH
ts-node evm/consensus-auth.js verify -e $ENV -n $CHAIN --authModule $AUTH
```

- `seed` copies the last 15 operator sets from the live auth (`--copyEpochs 15`, `--lookbackDays 30`).
  If the RPC limits `eth_getLogs` ranges, lower `--logChunkSize`.
- `verify` warns that the deployer still owns the auth. That is expected at this point.
- If `verify` fails, do not continue. Re-seed and verify again.

### Step 5: Execute the upgrade (after ETA)

```fish
ts-node evm/governance.js execute -e $ENV -n $CHAIN --target $GATEWAY --calldata $UPGRADE_CALLDATA
```

`execute` first runs the auth-readiness guard and aborts if the auth is not ready. Do not bypass it.
On success, config sets `AxelarGateway.authModule = $AUTH` and removes `pendingAuthModule`.

### Step 5b: Execute the pauser transfer (after Step 5)

```fish
ts-node evm/governance.js execute -e $ENV -n $CHAIN --target $GATEWAY --calldata $PAUSER_CALLDATA
ts-node evm/gateway.js -e $ENV -n $CHAIN --action pauser
```

Until this runs, `pauser()` is `0x0`. During that window, only governance can pause the gateway.

### Step 6: Hand auth ownership to the proxy (right after Step 5)

```fish
ts-node evm/consensus-auth.js handoff -e $ENV -n $CHAIN --authModule $AUTH
ts-node evm/consensus-auth.js verify  -e $ENV -n $CHAIN --authModule $AUTH
```

`handoff` refuses a stale seed and asserts that ownership landed on the proxy. The final `verify` must pass
(owned by the proxy, seeded, live set within retention).

### Step 7: Commit the config

Commit the updated `axelar-chains-config/info/testnet.json`. It should change `implementation`,
`implementationCodehash` and `authModule`, and have no `pendingAuthModule` left.

## Rest of testnet

Once the pilot passes the checklist, repeat Steps 1–7 for the remaining chains. Pass a comma-separated list to `-n` so
that Step 3 builds a single proposal. Run Step 5 per chain, using each chain's target and calldata.

```fish
set -x CHAIN ethereum-sepolia,avalanche,moonbeam,binance,kava,filecoin-2,scroll,immutable,arbitrum-sepolia,optimism-sepolia,base-sepolia,mantle-sepolia,polygon-sepolia
```

- All batch chains use the same `ConstAddressDeployer` (`0x98B2920D…`) and `Create3Deployer` (`0x6513Aedb…`), so with
  deployer `0xBF6d…` every chain gets the same `$AUTH` and `$IMPL` as the pilot. Fund `0xBF6d…` on every chain first.
- `$GATEWAY` is not shared: `avalanche`, `moonbeam`, `binance`, `kava`, `filecoin-2` and `scroll` use a different gateway
  proxy from the other chains. In Step 3b pass each chain's gateway, and in Steps 5 and 5b take each chain's target and
  calldata from the Step 3 output.

## Failure handling

| Situation                                    | Action                                                                                               |
| -------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Proposal rejected or not relayed             | Re-schedule (Step 3). `AUTH` and `IMPL` can be reused.                                               |
| `execute` run before ETA                     | It aborts and prints the ETA. Wait, then retry.                                                      |
| `verify` fails before execute                | Re-run `seed`, then `verify`. Do not execute until it passes.                                        |
| `execute` aborts on the readiness guard      | Re-seed and retry. Do not bypass the guard.                                                          |
| ETA slipped and the seed aged out            | Re-run `seed` (repeatable), then `verify`.                                                           |
| `transferPauser` executed before the upgrade | It reverts on the old implementation and the timelock stays scheduled. Run Step 5 first, then retry. |
| `handoff` fails                              | The gateway keeps working on the seeded sets. Re-run `handoff` within 16 rotations.                  |

## Checklist

- [ ] Gateway `implementation()` returns `$IMPL`.
- [ ] Gateway `authModule()` returns `$AUTH`.
- [ ] `$AUTH` `owner()` is the gateway proxy, and `consensus-auth verify` passes.
- [ ] Gateway `pauser()` returns `$PAUSER`, and `paused()` returns `false`.
- [ ] A GMP call with `$CHAIN` as destination is approved and executed.
- [ ] A GMP call with `$CHAIN` as source is relayed.
- [ ] After the next daily rotation, `$AUTH` `currentEpoch()` has advanced.
