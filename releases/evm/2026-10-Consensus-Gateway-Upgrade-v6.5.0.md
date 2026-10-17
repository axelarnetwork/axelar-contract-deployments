# Consensus Gateway Upgrade + Auth Module Retrofit v6.5.0

|                | **Owner**                           |
| -------------- | ----------------------------------- |
| **Created By** | @jakovmitrovski, @h4writer, @dolfje |
| **Deployment** | @h4writer <hannes@commonprefix.com> |

| **Environment**      | **Chain**                           | **Deployment Status**                    | **Date**   |
| -------------------- | ----------------------------------- | ---------------------------------------- | ---------- |
| **Devnet Amplifier** | `avalanche`, `ethereum`, `optimism` | Completed (Jakov)                        | 2026-09-01 |
| **Stagenet**         | all 9 active consensus chains       | Completed (Jakov)                        | 2026-09-02 |
| **Testnet**          | `linea-sepolia` (pilot)             | Completed (#656, #657)                   | 2026-10-13 |
|                      | `ethereum-sepolia`                  | Completed (#660, #661); rotation pending | 2026-10-17 |
|                      | `avalanche`                         | Completed (#658, #659)                   | 2026-10-16 |
|                      | `binance`                           | Completed (#658, #659)                   | 2026-10-16 |
|                      | `kava`                              | Completed (#658, #659)                   | 2026-10-16 |
|                      | `filecoin-2`                        | Completed (#658, #659)                   | 2026-10-16 |
|                      | `immutable`                         | Completed (#658, #659)                   | 2026-10-16 |
|                      | `arbitrum-sepolia`                  | Completed (#658, #659)                   | 2026-10-16 |
|                      | `optimism-sepolia`                  | Completed (#658, #659)                   | 2026-10-16 |
|                      | `base-sepolia`                      | Completed (#658, #659)                   | 2026-10-16 |
|                      | `mantle-sepolia`                    | Completed (#658, #659)                   | 2026-10-16 |
|                      | `polygon-sepolia`                   | Completed (#658, #659)                   | 2026-10-16 |
|                      | `scroll`                            | Skipped (deactivated, #634)              | -          |
|                      | `moonbeam`                          | Skipped (deactivated, #635)              | -          |
| **Mainnet**          | -                                   | -                                        | TBD        |

- [Runbook](https://app.notion.com/p/commonprefix/Consensus-Gateway-Upgrade-3cdbf672de8281529199d4cf4f065d4a)
- [axelar-cgp-solidity v6.5.0](https://github.com/axelarnetwork/axelar-cgp-solidity/releases/tag/v6.5.0)

## Background

Upgrade the consensus (legacy) `AxelarGateway` implementation to cgp-solidity `v6.5.0`, which binds to a new
`AxelarAuthWeighted`. `authModule` is a constructor `immutable` on the implementation, so the upgrade needs a
replacement auth module. The retrofit works like this:

1. Deploy the auth **unseeded** at a CREATE2 address, owned by the deployer.
2. Deploy the implementation bound to that auth.
3. Get an operator proposal for the upgrade through Axelar governance and relay it to `AxelarServiceGovernance` (ASG).
4. Seed the auth with the recent operator sets **just before** execution.
5. Execute the upgrade with the ASG operator key.
6. Hand auth ownership to the gateway proxy **right after**.
7. Set the pauser. 6.5.0 adds a `pauser` role that can pause the gateway without the timelock. `setup()` does not set
   it, so after the upgrade `pauser()` is `0x0` until governance calls `transferPauser`.

Why the timing matters: `validateProof` only accepts operator sets within `OLD_KEY_RETENTION = 16` epochs of the
newest one, and operators rotate about once a day. If you seed too early, the signing set ages out before execution.
Between execute and handoff the seeder owns the auth, so new rotations do not register in it. After 16 rotations
(about 16 days) every batch reverts, and `transferOperatorship` cannot fix that because it arrives inside a batch.

### Scope

| Network     | Chains                                                                                                                                                                               |
| ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Testnet** | `linea-sepolia` first, as a pilot. Then `ethereum-sepolia,avalanche,binance,kava,filecoin-2,immutable,arbitrum-sepolia,optimism-sepolia,base-sepolia,mantle-sepolia,polygon-sepolia` |

Out of scope on testnet:

- `blast-sepolia`, `centrifuge-2`: on-chain `governance()` is `0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`, not the ASG.
- `fantom`: RPC is dead.
- `moonbeam`, `scroll`: deactivated on Axelar (proposals #635 and #634), so they get no batches or operator rotations.

Before including a chain, check on-chain that `governance()` returns the ASG (`0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`).

### Pilot: linea-sepolia

| Item                    | Value                                                                                |
| ----------------------- | ------------------------------------------------------------------------------------ |
| Gateway proxy           | `0xe432150cce91c13a887f7D836923d5597adD8E31`                                         |
| Current implementation  | `0xc1712652326E87D193Ac11910934085FF45C2F48`                                         |
| Current auth module     | `0x1a920B29eBD437074225cAeE44f78FC700B27a5d`                                         |
| AxelarServiceGovernance | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525`                                         |
| ASG operator            | `0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e`                                         |
| Deployer / auth seeder  | `0xBF6d81aDCfCa1D7Bea60E23C1da5483F84E4A668` ([Testnet] Deployer EOA: EVM)           |
| Pauser (new role)       | `0xF140b31cBe9F6a3b1d52491b84bABEDAa2962faB` ([Testnet] Emergency Operator EOA: EVM) |
| Governance proposer     | `axelar1wz8vvty9g56g43yrzgapv6ue5gsm4ggrxvhww3` ([Testnet] Deployer EOA: Axelar)     |
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

2. Set the shell variables (fish). `PRIVATE_KEY` is the deployer `0xBF6d…`, and `MNEMONIC` is the governance proposer
   `axelar1wz8vvty9g56g43yrzgapv6ue5gsm4ggrxvhww3` ([Testnet] Deployer EOA: Axelar). The scripts read both from the
   environment. An exported variable takes precedence over `.env`, but if you forget to export it, a `PRIVATE_KEY` left
   in `.env` is used silently. Steps 5 and 7 execute with the ASG operator key instead: switch `PRIVATE_KEY` for those
   commands only.

```fish
read -sx PRIVATE_KEY
read -sx MNEMONIC
set -x ENV testnet
set -x CHAIN linea-sepolia

set GATEWAY 0xe432150cce91c13a887f7D836923d5597adD8E31
set ASG 0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525
set DEPLOYER 0xBF6d81aDCfCa1D7Bea60E23C1da5483F84E4A668
set AUTH_SALT "AxelarAuthWeighted v6.5.0 testnet"
set IMPL_SALT "AxelarGateway v6.5"
set AUTH 0x96e4D329fd6099b40446d660b3A72CA9aCC94EC6 # predicted, confirmed in Step 1
set IMPL 0x53A68B407392F8A39cc6418c939f4d9cb75350D7 # predicted, confirmed in Step 2
set PAUSER 0xF140b31cBe9F6a3b1d52491b84bABEDAa2962faB
set PAUSER_CALLDATA 0x4421ea21000000000000000000000000f140b31cbe9f6a3b1d52491b84babedaa2962fab # transferPauser($PAUSER)
```

3. Check that the deployer is funded on `$CHAIN`, that the proposer holds at least 3,000 AXL plus fees, and that the
   gateway is governed by the ASG:

```fish
cast call $GATEWAY "governance()(address)" --rpc-url https://rpc.sepolia.linea.build
curl -s https://axelar-testnet-lcd.qubelabs.io/cosmos/bank/v1beta1/balances/axelar1wz8vvty9g56g43yrzgapv6ue5gsm4ggrxvhww3/by_denom?denom=uaxl
```

The proposer only holds enough for one expedited deposit at a time, so the upgrade (Step 3) and the pauser transfer
(Step 7) are separate proposals that never run at the same time. The deposit of each passed proposal is refunded.

## Deployment Steps

### Step 1: Deploy the unseeded auth module

```fish
npx ts-node evm/consensus-auth.js predict -e $ENV -n $CHAIN -s $AUTH_SALT
npx ts-node evm/consensus-auth.js deploy  -e $ENV -n $CHAIN -s $AUTH_SALT
```

Check that the printed auth address equals `$AUTH`. It stays unseeded and owned by the deployer until Steps 4 and 6.

### Step 2: Deploy the implementation bound to `$AUTH`

```fish
npx ts-node evm/deploy-consensus-gateway.js -e $ENV -n $CHAIN \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT
```

- The script prints `Contract already deployed at predicted address "$GATEWAY"!` before the confirmation prompt. This
  is expected with `--reuseProxy` and can be ignored: it only checks that the proxy exists. Check that the address is
  `$GATEWAY` and answer `y`.
- Ownership transfer to the proxy is skipped, and config records `$AUTH` as `AxelarGateway.pendingAuthModule`.
- Check that the printed implementation address equals `$IMPL`.
- Do **not** pass `--upgrade`. The gateway is ASG-governed, so the upgrade has to go through governance.

### Step 3: Schedule the upgrade through governance

Submit as an **expedited** proposal, which is the `governance.js` default. On testnet that means a 300s voting period,
a 66.7% threshold and a 3,000 AXL deposit, refunded unless vetoed.

```fish
npx ts-node evm/governance.js schedule-operator upgrade 0 -e $ENV -n $CHAIN \
  --targetContractName AxelarGateway --implementation $IMPL \
  --title "Upgrade $CHAIN consensus gateway to v6.5.0" \
  --description "Upgrades the $CHAIN AxelarGateway to $IMPL, bound to the new AxelarAuthWeighted $AUTH"
```

- The output warns that `0 is less than the minimum eta`. That is expected: the ASG ignores the activation time of an
  operator proposal.
- Check that the output shows `Setup Params for upgrading: 0x`. A non-empty value means the upgrade would also change
  the gateway's governance or mint limiter. In that case, stop. Do not pass `--newGovernance` / `--newMintLimiter`.
- `Governance target (for execute-operator-proposal)` must equal `$GATEWAY`.
- Save the value of `Governance calldata (for execute-operator-proposal)` for Step 5. It starts with `0xa3499c73`
  (`upgrade`).

```fish
set UPGRADE_CALLDATA <value of 'Governance calldata (for execute-operator-proposal)'>
```

- With `MNEMONIC` set, the proposal is submitted directly. To review it first, use `--generate-only proposal.json`.

Wait for the proposal to pass. That approves the call on `$GATEWAY`, but nothing calls the ASG automatically (for
the pilot, proposal #656 stayed approved on the gateway without reaching the ASG), so relay it yourself. Take the
proposal id from the `schedule-operator` output, or from
`https://axelar-testnet-lcd.qubelabs.io/cosmos/gov/v1/proposals?pagination.reverse=true&pagination.limit=1`.

```fish
set PROPOSAL_ID <proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n $CHAIN
```

- `relay` reads the command id and payload from the Axelar block that executed the proposal. Check that the printed
  type is `ApproveOperator`, the target is `$GATEWAY` and the calldata is `$UPGRADE_CALLDATA` before confirming. The
  relay should emit `OperatorProposalApproved`. Anyone can send it; it costs about 75k gas.
- `relay` prints `Proposal eta: 1970-01-01T00:00:00`. That is expected: it is the `0` activation time from the payload.
- If it reports that the call is not approved on the gateway yet, the command batch hasn't reached `$GATEWAY`. Wait a
  few minutes and retry.

Check that the upgrade is approved:

```fish
cast call $ASG "isOperatorProposalApproved(address,bytes,uint256)(bool)" $GATEWAY $UPGRADE_CALLDATA 0 \
  --rpc-url https://rpc.sepolia.linea.build
```

### Step 4: Seed the auth and verify

Run this right before execution, once the proposal has relayed. If execution slips by more than a few rotations,
re-run `seed`.

```fish
npx ts-node evm/consensus-auth.js seed   -e $ENV -n $CHAIN --authModule $AUTH
npx ts-node evm/consensus-auth.js verify -e $ENV -n $CHAIN --authModule $AUTH
```

- `seed` copies the last 15 operator sets from the live auth (`--copyEpochs 15`, `--lookbackDays 30`).
  If the RPC limits `eth_getLogs` ranges, lower `--logChunkSize`.
- `verify` warns that the deployer still owns the auth. That is expected at this point.
- If `verify` fails, do not continue. Re-seed and verify again.

### Step 5: Execute the upgrade with the operator key

Run this with the ASG operator key (`0x6e079CD1…` on testnet) as `PRIVATE_KEY`. With any other key the transaction
reverts (`NotAuthorized`; the script only shows `CALL_EXCEPTION`). The auth seeder is still the deployer `0xBF6d…`, so
pass it with `--authSeeder`.

```fish
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $CHAIN \
  --target $GATEWAY --calldata $UPGRADE_CALLDATA --authSeeder $DEPLOYER
```

`execute-operator-proposal` first runs the auth-readiness guard and aborts if the auth is not ready. Do not bypass it.
On success, config sets `AxelarGateway.authModule = $AUTH` and removes `pendingAuthModule`.

### Step 6: Hand auth ownership to the proxy (right after Step 5)

Switch `PRIVATE_KEY` back to the deployer: it owns `$AUTH` until the handoff.

```fish
npx ts-node evm/consensus-auth.js handoff -e $ENV -n $CHAIN --authModule $AUTH
npx ts-node evm/consensus-auth.js verify  -e $ENV -n $CHAIN --authModule $AUTH
```

`handoff` refuses a stale seed and asserts that ownership landed on the proxy. The final `verify` must pass
(owned by the proxy, seeded, live set within retention).

### Step 7: Set the pauser

Run this only after Step 5 has executed and the deposit of the Step 3 proposal has been refunded. `transferPauser`
only exists on the new implementation. Until this step completes, `pauser()` is `0x0`, so only governance can pause
the gateway.

As in Step 3, use an operator proposal with activation time `0`. The "less than the minimum eta" warning and the
`1970-01-01` ETA printed by `relay` are expected.

```fish
npx ts-node evm/governance.js schedule-operator raw 0 -e $ENV -n $CHAIN \
  --target $GATEWAY --calldata $PAUSER_CALLDATA \
  --title "Set $CHAIN consensus gateway pauser" \
  --description "Calls transferPauser($PAUSER) on the $CHAIN AxelarGateway"
```

As in Step 3, wait for the proposal to pass, then relay it with the pauser proposal's id. Check that the printed type is
`ApproveOperator`, the target is `$GATEWAY` and the calldata is `$PAUSER_CALLDATA`. The relay should emit
`OperatorProposalApproved`.

```fish
set PROPOSAL_ID <pauser proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n $CHAIN
```

Execute with the ASG operator key, as in Step 5.

```fish
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $CHAIN           --target $GATEWAY --calldata $PAUSER_CALLDATA
npx ts-node evm/gateway.js -e $ENV -n $CHAIN --action pauser
```

### Step 8: Commit the config

Commit the updated `axelar-chains-config/info/testnet.json`. It should change `implementation`,
`implementationCodehash` and `authModule`, and have no `pendingAuthModule` left.

## Rest of testnet

Once the pilot passes the checklist, run the same steps for the remaining chains in one batch. The checks and
expected warnings are the same as in the pilot step of the same number; this section only lists what differs.

- All batch chains use the same `ConstAddressDeployer` (`0x98B2920D…`) and `Create3Deployer` (`0x6513Aedb…`), so with
  deployer `0xBF6d…` every chain gets the same `$AUTH` and `$IMPL` as the pilot.
- `ethereum-sepolia` is not in `$CHAINS`: its implementation deploy failed. See
  [Fix ethereum-sepolia](#fix-ethereum-sepolia).
- The gateway proxy is not shared. `avalanche`, `binance`, `filecoin-2` and `arbitrum-sepolia` each have their own,
  `kava` and `mantle-sepolia` share one, and the other chains use `0xe432150c…`. The commands below take each chain's
  gateway and RPC from the config.
- `-n $CHAINS` commands handle all chains in one go. Commands that need a chain's own gateway loop over the list.
- `filecoin-2` is not in `$CHAINS`: its transactions need explicit fees (`$FILECOIN_GAS`), and `--gasOptions` applies
  to every chain in `-n`. Each step has a separate filecoin-2 command. The proposals (Steps 3 and 7) and the read-only
  checks include it through `$CHAINS,filecoin-2`.

### Variables

The same values as the pilot, with `CHAINS` instead of `CHAIN`, and without `GATEWAY` and `UPGRADE_CALLDATA`: they
differ per chain, so the commands below read them from the config. `cast` reads an exported `CHAIN` as its `--chain`
option, so a leftover comma-separated `CHAIN` makes `cast` fail and a single one makes `cast send` sign for the wrong
chain id.

```fish
set -e CHAIN GATEWAY UPGRADE_CALLDATA
read -sx PRIVATE_KEY
read -sx MNEMONIC
set -x ENV testnet
set -x CHAINS avalanche,binance,kava,immutable,arbitrum-sepolia,optimism-sepolia,base-sepolia,mantle-sepolia,polygon-sepolia

set ASG 0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525
set DEPLOYER 0xBF6d81aDCfCa1D7Bea60E23C1da5483F84E4A668
set AUTH_SALT "AxelarAuthWeighted v6.5.0 testnet"
set IMPL_SALT "AxelarGateway v6.5"
set AUTH 0x96e4D329fd6099b40446d660b3A72CA9aCC94EC6 # same as the pilot, confirmed in Step 1
set IMPL 0x53A68B407392F8A39cc6418c939f4d9cb75350D7 # same as the pilot, confirmed in Step 2
set PAUSER 0xF140b31cBe9F6a3b1d52491b84bABEDAa2962faB
set PAUSER_CALLDATA 0x4421ea21000000000000000000000000f140b31cbe9f6a3b1d52491b84babedaa2962fab # transferPauser($PAUSER)
# without explicit fees ethers reserves gasLimit × 1.5 gwei (about 0.45 FIL) per filecoin-2 transaction
set FILECOIN_GAS '{"gasLimit":300000000,"maxFeePerGas":2000000,"maxPriorityFeePerGas":1000000}'

# cfg <chain> <field>: a field of that chain in the config, e.g. cfg kava .rpc
function cfg
    jq -r ".chains[\"$argv[1]\"]$argv[2]" axelar-chains-config/info/$ENV.json
end
```

Fund the deployer `0xBF6d…` and the ASG operator `0x6e079CD1…` on every chain, then check the balances and that every
gateway is governed by `$ASG`:

```fish
for c in (string split , $CHAINS,filecoin-2)
    set rpc (cfg $c .rpc)
    echo "$c deployer="(cast balance $DEPLOYER -e --rpc-url $rpc)" operator="(cast balance 0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e -e --rpc-url $rpc)" governance="(cast call (cfg $c .contracts.AxelarGateway.address) "governance()(address)" --rpc-url $rpc)
end
```

### Steps 1–2: Deploy the auth and the implementation

```fish
npx ts-node evm/consensus-auth.js predict -e $ENV -n $CHAINS,filecoin-2 -s $AUTH_SALT
npx ts-node evm/consensus-auth.js deploy  -e $ENV -n $CHAINS -s $AUTH_SALT
npx ts-node evm/consensus-auth.js deploy  -e $ENV -n filecoin-2 -s $AUTH_SALT --gasOptions $FILECOIN_GAS

npx ts-node evm/deploy-consensus-gateway.js -e $ENV -n $CHAINS \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT
npx ts-node evm/deploy-consensus-gateway.js -e $ENV -n filecoin-2 \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT --gasOptions $FILECOIN_GAS
```

The "Contract already deployed at predicted address" warning shows each chain's own gateway.

### Step 3: Schedule the upgrade and relay

`schedule-operator` sets the new mint limiter to the config's `Multisig` address unless the gateway already uses it.
Immutable's gateway uses `0xB8Cd93C83A974649D76B1c19f311f639e62272BC` as mint limiter, not the shared Multisig
`0xCC940AE4…`, so its upgrade would also change the mint limiter (non-empty `Setup Params`). `--newMintLimiter` can't fix
this, because it applies to every chain in `-n`. Point immutable's config `Multisig` at its live mint limiter for this
command, and switch it back afterwards:

```fish
set -l f axelar-chains-config/info/$ENV.json
jq '.chains.immutable.contracts.Multisig.address = "0xB8Cd93C83A974649D76B1c19f311f639e62272BC"' $f >$f.tmp; and mv $f.tmp $f

npx ts-node evm/governance.js schedule-operator upgrade 0 -e $ENV -n $CHAINS,filecoin-2 \
  --targetContractName AxelarGateway --implementation $IMPL \
  --title "Upgrade consensus gateways to v6.5.0 ($CHAINS,filecoin-2)" \
  --description "Upgrades the AxelarGateway on $CHAINS,filecoin-2 to $IMPL, bound to the new AxelarAuthWeighted $AUTH"

jq '.chains.immutable.contracts.Multisig.address = "0xCC940AE49C78F20E3F13F3cF37e996b98Ac3EC68"' $f >$f.tmp; and mv $f.tmp $f
```

For every chain, including immutable, check `Setup Params for upgrading: 0x` and that the target is that chain's
gateway. If any chain shows a non-empty value, answer `n`. The calldata does
not need to be saved: the commands below rebuild it from the `implementation` and `implementationCodehash` that Step 2
wrote to the config.

Once the proposal has passed, relay it to all chains and check the approvals:

```fish
set PROPOSAL_ID <proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n (string join , (string split , $CHAINS | string match -v avalanche))
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n avalanche --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n filecoin-2 --gasOptions $FILECOIN_GAS

for c in (string split , $CHAINS,filecoin-2)
    set calldata (cast calldata "upgrade(address,bytes32,bytes)" (cfg $c .contracts.AxelarGateway.implementation) (cfg $c .contracts.AxelarGateway.implementationCodehash) 0x)
    echo "$c approved="(cast call $ASG "isOperatorProposalApproved(address,bytes,uint256)(bool)" (cfg $c .contracts.AxelarGateway.address) $calldata 0 --rpc-url (cfg $c .rpc))
end
```

### Step 4: Seed the auth and verify

`binance` and `filecoin-2` are seeded separately. They can't copy the history from logs: binance's RPCs don't serve
`OperatorshipTransferred` logs older than about a day (publicnode returns them empty, without an error), and
filecoin-2's RPC refuses lookbacks over 14 days. `--copyEpochs` would find too few operator sets, and batches still
signed by an older set would revert after the upgrade. Instead, take the previous sets from Axelar by key ID.
`prev_key_ids` walks a chain's batches back from the newest one and collects the 14 keys before the current one, oldest
first:

```fish
function prev_key_ids --argument-names chain
    set -l lcd (jq -r .axelar.lcd axelar-chains-config/info/$ENV.json)
    set -l current (curl -s $lcd/axelar/multisig/v1beta1/key_id/$chain | jq -r .key_id)
    set -l keys
    set -l id ''
    while test (count $keys) -lt 14
        set -l batch (curl -s "$lcd/axelar/evm/v1beta1/batched_commands/$chain/$id")
        set -l key (echo $batch | jq -r .key_id)
        if test "$key" != "$current"; and not contains -- $key $keys
            set -a keys $key
        end
        set id (echo $batch | jq -r .prev_batched_commands_id)
        test -n "$id" -a "$id" != null; or break
    end
    string join , $keys[-1..1]
end

set BINANCE_PREV_KEY_IDS (prev_key_ids binance)
set FILECOIN_PREV_KEY_IDS (prev_key_ids filecoin-2)
echo $BINANCE_PREV_KEY_IDS
echo $FILECOIN_PREV_KEY_IDS
```

Run it right before seeding: keys rotate daily. Each should print 14 key IDs, `evm-<chain>-<height>` in increasing
order. `seed` adds the current key itself.

The other chains copy the history from logs, 10,000 blocks per request by default. Two RPCs need adjusting first:

- `immutable` allows at most 5,000 blocks per `eth_getLogs`, so its seed uses `--logChunkSize 5000`.
- `avalanche` (`https://api.avax-test.network/ext/bc/C/rpc`) and `base-sepolia` (`https://sepolia.base.org`) allow only
  about 100 blocks, so 30 days would take over 10,000 requests. Point them at publicnode in the config for the seed,
  which allows 50,000 blocks and serves the full 30 days, and switch them back afterwards. Avalanche's publicnode
  endpoint once returned a gas estimate of 165M for a seed transaction (the node rejected it with "exceeds block gas
  limit"; the real cost is about 156k), so avalanche gets a fixed gas limit.

```fish
function set_rpc --argument-names chain rpc
    set -l f axelar-chains-config/info/$ENV.json
    jq --arg c $chain --arg r $rpc '.chains[$c].rpc = $r' $f >$f.tmp; and mv $f.tmp $f
end
set_rpc avalanche https://avalanche-fuji-c-chain-rpc.publicnode.com
set_rpc base-sepolia https://base-sepolia-rpc.publicnode.com
```

```fish
npx ts-node evm/consensus-auth.js seed -e $ENV -n avalanche        --authModule $AUTH --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/consensus-auth.js seed -e $ENV -n binance          --authModule $AUTH --copyEpochs 0 --prevKeyIDs $BINANCE_PREV_KEY_IDS
npx ts-node evm/consensus-auth.js seed -e $ENV -n kava             --authModule $AUTH
npx ts-node evm/consensus-auth.js seed -e $ENV -n filecoin-2       --authModule $AUTH --copyEpochs 0 --prevKeyIDs $FILECOIN_PREV_KEY_IDS --gasOptions $FILECOIN_GAS
npx ts-node evm/consensus-auth.js seed -e $ENV -n immutable        --authModule $AUTH --logChunkSize 5000
npx ts-node evm/consensus-auth.js seed -e $ENV -n arbitrum-sepolia --authModule $AUTH
npx ts-node evm/consensus-auth.js seed -e $ENV -n optimism-sepolia --authModule $AUTH
npx ts-node evm/consensus-auth.js seed -e $ENV -n base-sepolia     --authModule $AUTH
npx ts-node evm/consensus-auth.js seed -e $ENV -n mantle-sepolia   --authModule $AUTH
npx ts-node evm/consensus-auth.js seed -e $ENV -n polygon-sepolia  --authModule $AUTH

npx ts-node evm/consensus-auth.js verify -e $ENV -n $CHAINS,filecoin-2 --authModule $AUTH
```

`seed` should report 15 sets on every chain and end at epoch 15. A warning that it found fewer than 15 sets means the
RPC did not serve the history: don't continue on that chain. `verify` refuses to run on a chain while a key rotation is
in progress. Wait for it to land and re-run.

Switch the two RPCs back once every chain is seeded:

```fish
set_rpc avalanche https://api.avax-test.network/ext/bc/C/rpc
set_rpc base-sepolia https://sepolia.base.org
```

### Step 5: Execute the upgrade (operator key)

Run this with the ASG operator key as `PRIVATE_KEY`, one chain at a time, each right after that chain's seed has
verified. The calldata is `upgrade($IMPL, codehash, 0x)`. The codehash differs per chain because the implementation's
`tokenDeployer` immutable differs: `UPGRADE_A` for the chains on the `0x99B5FA03…` implementation, `UPGRADE_B` for the
rest. Both match the calls approved by the proposal. The config gives optimism-sepolia and base-sepolia a fixed 8M gas
limit, which with ethers' default 1.5 gwei priority fee needs 0.012 ETH up front, more than the operator's 0.01 ETH, so
their lines lower it to 1M (the call needs about 55k). Check that the script prints
`Wallet address: 0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e`; with any other key the transaction reverts.

```fish
read -sx PRIVATE_KEY    # the ASG operator key, 0x6e079CD1…

set UPGRADE_A 0xa3499c7300000000000000000000000053a68b407392f8a39cc6418c939f4d9cb75350d7bcc18fe1ccd51cb93a75d2166e8cb7bdd5ba8ef62d4c27d64f3ed35ee3e6222200000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000
set UPGRADE_B 0xa3499c7300000000000000000000000053a68b407392f8a39cc6418c939f4d9cb75350d7aafd4d842b2e3f8008d9dfc1a6ab6f99ac19460e86b05daaf1f54ca1391e9f3e00000000000000000000000000000000000000000000000000000000000000600000000000000000000000000000000000000000000000000000000000000000

npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n avalanche        --target 0xC249632c2D40b9001FE907806902f63038B737Ab --calldata $UPGRADE_A --authSeeder $DEPLOYER --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n binance          --target 0x4D147dCb984e6affEEC47e44293DA442580A3Ec0 --calldata $UPGRADE_A --authSeeder $DEPLOYER
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n kava             --target 0xC8D18F85cB0Cee5C95eC29c69DeaF6cea972349c --calldata $UPGRADE_A --authSeeder $DEPLOYER
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n filecoin-2       --target 0x999117D44220F33e0441fbAb2A5aDB8FF485c54D --calldata $UPGRADE_A --authSeeder $DEPLOYER --gasOptions $FILECOIN_GAS
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n immutable        --target 0xe432150cce91c13a887f7D836923d5597adD8E31 --calldata $UPGRADE_B --authSeeder $DEPLOYER
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n arbitrum-sepolia --target 0xe1cE95479C84e9809269227C7F8524aE051Ae77a --calldata $UPGRADE_B --authSeeder $DEPLOYER
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n optimism-sepolia --target 0xe432150cce91c13a887f7D836923d5597adD8E31 --calldata $UPGRADE_B --authSeeder $DEPLOYER --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n base-sepolia     --target 0xe432150cce91c13a887f7D836923d5597adD8E31 --calldata $UPGRADE_B --authSeeder $DEPLOYER --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n mantle-sepolia   --target 0xC8D18F85cB0Cee5C95eC29c69DeaF6cea972349c --calldata $UPGRADE_B --authSeeder $DEPLOYER
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n polygon-sepolia  --target 0xe432150cce91c13a887f7D836923d5597adD8E31 --calldata $UPGRADE_B --authSeeder $DEPLOYER
```

`execute-operator-proposal` first checks that the operator proposal is approved, then runs the auth-readiness guard,
and aborts if either fails. Do not bypass it. On success, config sets `AxelarGateway.authModule = $AUTH` and removes
`pendingAuthModule` for that chain.

### Step 6: Hand auth ownership to the proxy (deployer key)

```fish
read -sx PRIVATE_KEY    # the deployer, 0xBF6d…
npx ts-node evm/consensus-auth.js handoff -e $ENV -n (string join , (string split , $CHAINS | string match -v avalanche)) --authModule $AUTH
npx ts-node evm/consensus-auth.js handoff -e $ENV -n avalanche --authModule $AUTH --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/consensus-auth.js handoff -e $ENV -n filecoin-2 --authModule $AUTH --gasOptions $FILECOIN_GAS
npx ts-node evm/consensus-auth.js verify  -e $ENV -n $CHAINS,filecoin-2 --authModule $AUTH
```

### Step 7: Set the pauser

`--targetContractName AxelarGateway` instead of `--target` takes each chain's own gateway as the target.

```fish
npx ts-node evm/governance.js schedule-operator raw 0 -e $ENV -n $CHAINS,filecoin-2 \
  --targetContractName AxelarGateway --calldata $PAUSER_CALLDATA \
  --title "Set consensus gateway pauser ($CHAINS,filecoin-2)" \
  --description "Calls transferPauser($PAUSER) on the AxelarGateway on $CHAINS,filecoin-2"
```

Once it has passed, relay it, then execute with the operator key and check the pauser. Avalanche (publicnode gas
estimate), optimism-sepolia and base-sepolia (the config's 8M gas limit needs 0.012 ETH up front, the operator has 0.01)
execute with a 1M gas limit:

```fish
set PROPOSAL_ID <pauser proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n (string join , (string split , $CHAINS | string match -v avalanche))
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n avalanche --gasOptions '{"gasLimit":1000000}'
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n filecoin-2 --gasOptions $FILECOIN_GAS

read -sx PRIVATE_KEY    # the ASG operator key, 0x6e079CD1…
for c in (string split , $CHAINS | string match -v avalanche | string match -v optimism-sepolia | string match -v base-sepolia)
    npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $c \
      --target (cfg $c .contracts.AxelarGateway.address) --calldata $PAUSER_CALLDATA
end
for c in avalanche optimism-sepolia base-sepolia
    npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $c \
      --target (cfg $c .contracts.AxelarGateway.address) --calldata $PAUSER_CALLDATA --gasOptions '{"gasLimit":1000000}'
end
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n filecoin-2 \
  --target (cfg filecoin-2 .contracts.AxelarGateway.address) --calldata $PAUSER_CALLDATA --gasOptions $FILECOIN_GAS

npx ts-node evm/gateway.js -e $ENV -n $CHAINS,filecoin-2 --action pauser
```

### Step 8: Commit the config

As in the pilot, for every chain.

## Fix ethereum-sepolia

The batch deploy (Steps 1–2 of the rest of testnet) deployed the auth on `ethereum-sepolia`, but the implementation
deploy reverted (tx `0x0b5094c6afc86b7f16f0569c3c288cc7310dee9f1f7a013eb05d54490dc0cbfd`, 6.8M gas used). Since the
Sepolia fork of 2026-10-06, deploying code costs about 1,542 gas per byte instead of about 200, so the 18.7 KB
implementation needs about 30.8M gas, and the config's `gasLimit` for ethereum-sepolia is 8,000,000. The revert left
`$IMPL` free, the gateway on its old implementation, and the config unchanged. `deploy-consensus-gateway.js` did not
stop on the revert, so check the result on-chain.

Sepolia includes transactions above the 16,777,216 per-transaction cap (EIP-7825) when the extra gas pays for state
such as deployed code, so a higher gas limit is enough. Ethereum mainnet still charges about 200 gas per byte.

The batch proposals (#658, #659) went out without ethereum-sepolia, so it gets its own. Use the variables from the
rest of testnet, with:

```fish
set -x CHAINS ethereum-sepolia
```

### Step 2: Deploy the implementation with a higher gas limit

`eth_estimateGas` gives 30,770,139; 34,000,000 leaves about 10% margin.

```fish
npx ts-node evm/deploy-consensus-gateway.js -e $ENV -n $CHAINS \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT \
  --gasOptions '{"gasLimit":34000000}'
```

Check that it landed, and that the config now records it:

```fish
set rpc (cfg ethereum-sepolia .rpc)
cast codesize $IMPL --rpc-url $rpc                                  # 18717
cast keccak (cast code $IMPL --rpc-url $rpc)                        # = cfg ethereum-sepolia .contracts.AxelarGateway.implementationCodehash
cast call $IMPL "authModule()(address)" --rpc-url $rpc              # = $AUTH
cfg ethereum-sepolia .contracts.AxelarGateway.pendingAuthModule     # = $AUTH
```

### Steps 3–8

The same commands as the rest of testnet, with `$CHAINS` set to `ethereum-sepolia` and without the filecoin-2 lines.
The other ethereum-sepolia transactions store little state, so the config's 8M `gasLimit` is enough for them.

Ethereum-sepolia has the same mint limiter as immutable (`0xB8Cd93C8…`, not the shared `Multisig`), so Step 3 would
also change it (see Step 3 of the rest of testnet). With a single chain in `-n`, pass the live mint limiter with
`--newMintLimiter` instead of editing the config: a value equal to the live one is dropped. Check that the output
shows `Setup Params for upgrading: 0x`.

```fish
# Step 3 (deployer key, MNEMONIC set)
npx ts-node evm/governance.js schedule-operator upgrade 0 -e $ENV -n $CHAINS \
  --targetContractName AxelarGateway --implementation $IMPL \
  --newMintLimiter 0xB8Cd93C83A974649D76B1c19f311f639e62272BC \
  --title "Upgrade ethereum-sepolia consensus gateway to v6.5.0" \
  --description "Upgrades the AxelarGateway on ethereum-sepolia to $IMPL, bound to the new AxelarAuthWeighted $AUTH"
set PROPOSAL_ID <proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n $CHAINS

# Step 4 (deployer key)
npx ts-node evm/consensus-auth.js seed   -e $ENV -n $CHAINS --authModule $AUTH
npx ts-node evm/consensus-auth.js verify -e $ENV -n $CHAINS --authModule $AUTH

# Step 5 (operator key)
read -sx PRIVATE_KEY    # the ASG operator key, 0x6e079CD1…
set calldata (cast calldata "upgrade(address,bytes32,bytes)" (cfg ethereum-sepolia .contracts.AxelarGateway.implementation) (cfg ethereum-sepolia .contracts.AxelarGateway.implementationCodehash) 0x)
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $CHAINS \
  --target (cfg ethereum-sepolia .contracts.AxelarGateway.address) --calldata $calldata --authSeeder $DEPLOYER

# Step 6 (deployer key)
read -sx PRIVATE_KEY    # the deployer, 0xBF6d…
npx ts-node evm/consensus-auth.js handoff -e $ENV -n $CHAINS --authModule $AUTH
npx ts-node evm/consensus-auth.js verify  -e $ENV -n $CHAINS --authModule $AUTH

# Step 7 (deployer key and MNEMONIC to schedule and relay, operator key to execute)
npx ts-node evm/governance.js schedule-operator raw 0 -e $ENV -n $CHAINS \
  --targetContractName AxelarGateway --calldata $PAUSER_CALLDATA \
  --title "Set ethereum-sepolia consensus gateway pauser" \
  --description "Calls transferPauser($PAUSER) on the AxelarGateway on ethereum-sepolia"
set PROPOSAL_ID <pauser proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n $CHAINS
read -sx PRIVATE_KEY    # the ASG operator key, 0x6e079CD1…
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $CHAINS \
  --target (cfg ethereum-sepolia .contracts.AxelarGateway.address) --calldata $PAUSER_CALLDATA
npx ts-node evm/gateway.js -e $ENV -n $CHAINS --action pauser
```

Step 8: commit the ethereum-sepolia config change.

## Failure handling

| Situation                               | Action                                                                              |
| --------------------------------------- | ----------------------------------------------------------------------------------- |
| Proposal rejected                       | Re-schedule (Step 3). `AUTH` and `IMPL` can be reused.                              |
| Proposal not relayed                    | Manually relay                                                                      |
| Proposal approved but wrong             | Do not execute. **!important: Cancel the approval**, then fix and re-schedule.      |
| `execute-operator-proposal` reverts     | Check that `PRIVATE_KEY` is the ASG operator key and that the proposal is relayed.  |
| `verify` fails before execute           | Re-run `seed`, then `verify`. Do not execute until it passes.                       |
| Readiness guard aborts execution        | Re-seed and retry. Do not bypass the guard.                                         |
| Execution slipped and the seed aged out | Re-run `seed` (repeatable), then `verify`.                                          |
| Proposer lacks the 3,000 AXL deposit    | Wait for the previous proposal's deposit to be refunded, or top up the proposer.    |
| `handoff` fails                         | The gateway keeps working on the seeded sets. Re-run `handoff` within 16 rotations. |

## Checklist

Run this for each chain after Step 7. Record the result in the status table at the top, and note any item that is
still pending there.

- [ ] Gateway `implementation()` returns `$IMPL`.
- [ ] Gateway `authModule()` returns `$AUTH`.
- [ ] `$AUTH` `owner()` is the gateway proxy, and `consensus-auth verify` passes.
- [ ] Gateway `pauser()` returns `$PAUSER`, and `paused()` returns `false`.
- [ ] A GMP call with the chain as destination is approved and executed.
- [ ] A GMP call with the chain as source is relayed.
- [ ] After the next daily rotation, `$AUTH` `currentEpoch()` has advanced.
