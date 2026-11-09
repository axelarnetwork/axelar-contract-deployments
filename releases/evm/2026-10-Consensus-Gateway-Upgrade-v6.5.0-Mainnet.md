# Consensus Gateway Upgrade + Auth Module Retrofit v6.5.0 (Mainnet)

|                | **Owner**                           |
| -------------- | ----------------------------------- |
| **Created By** | @jakovmitrovski, @h4writer, @dolfje |
| **Deployment** | @h4writer <hannes@commonprefix.com> |

| **Chain**    | **Deployment Status**                                                              | **Date** |
| ------------ | ---------------------------------------------------------------------------------- | -------- |
| `celo`       | -                                                                                  | TBD      |
| `ethereum`   | -                                                                                  | TBD      |
| `avalanche`  | -                                                                                  | TBD      |
| `polygon`    | -                                                                                  | TBD      |
| `binance`    | -                                                                                  | TBD      |
| `arbitrum`   | -                                                                                  | TBD      |
| `kava`       | -                                                                                  | TBD      |
| `filecoin`   | -                                                                                  | TBD      |
| `optimism`   | -                                                                                  | TBD      |
| `linea`      | -                                                                                  | TBD      |
| `base`       | -                                                                                  | TBD      |
| `mantle`     | -                                                                                  | TBD      |
| `scroll`     | -                                                                                  | TBD      |
| `immutable`  | -                                                                                  | TBD      |
| `fraxtal`    | -                                                                                  | TBD      |
| `blast`      | Out of scope: [deactivated on Axelar](../axelar/2026-10-Blast-Deactivate-Chain.md) | -        |
| `fantom`     | Out of scope: pre-rename `InterchainGovernance`, not ASG                           | -        |
| `moonbeam`   | Out of scope: deactivated on Axelar                                                | -        |
| `centrifuge` | Out of scope: deactivated on Axelar                                                | -        |

- [Testnet release](./2026-10-Consensus-Gateway-Upgrade-v6.5.0.md): background, the pilot, and what went wrong on testnet
- [Runbook](https://app.notion.com/p/commonprefix/Consensus-Gateway-Upgrade-3cdbf672de8281529199d4cf4f065d4a)
- [axelar-cgp-solidity v6.5.0](https://github.com/axelarnetwork/axelar-cgp-solidity/releases/tag/v6.5.0)

## Background

The same upgrade as on testnet: a new `AxelarGateway` implementation (cgp-solidity `v6.5.0`) bound to a new, unseeded
`AxelarAuthWeighted`, which is seeded with the recent operator sets just before the upgrade executes and handed to the
gateway proxy right after. See the [testnet release](./2026-10-Consensus-Gateway-Upgrade-v6.5.0.md#background) for why
the timing matters.

Like testnet, both proposals are **operator proposals** (`schedule-operator` → `execute-operator-proposal`). Once relayed,
the ASG operator executes right away instead of waiting for the 72h `minimumTimeDelay`, so the seed (Step 4) can run
just before execution. The operator can still wait; nothing executes without the operator key.

What differs from testnet:

- **Rotations are about every 3 days**, not daily. `OLD_KEY_RETENTION = 16` epochs is about 6 weeks, so there is more
  slack between seed, execute and handoff. It also means 30 days of logs hold only about 10 operator sets, so `seed`
  takes the previous sets from Axelar by key ID (`--prevKeyIDs`) on every chain instead of reading them from logs. Most
  mainnet RPCs also refuse 30-day log ranges.
- **Every chain's mint limiter is its config `Multisig`**, so `Setup Params` is `0x` without the immutable workaround
  used on testnet.
- **Proposals are expedited** (the `governance.js` default, no `--standardProposal`): a **400,000 AXL** deposit
  (refunded unless vetoed), a 1-day voting period and a 66.7% threshold. An expedited proposal that misses the 66.7%
  threshold is not rejected but converted to a standard one (3-day voting period from submission, 50% threshold).
- The ASG operator `0xc8a8399c…` is in use: it executed operator proposal #499 on optimism.

## Scope

| Item                    | Value                                                                                          |
| ----------------------- | ---------------------------------------------------------------------------------------------- |
| AxelarServiceGovernance | `0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525` (`minimumTimeDelay` 259200), on every chain       |
| ASG operator            | `0xc8a8399c3D1207f4e109673be7047604737c1D56`                                                   |
| Deployer / auth seeder  | `0xB1209B087e544785138784A50933d163A9087a7D`                                                   |
| Pauser (new role)       | `0x72164D4448fe6cfA472946feDC71e83b4628B1aF` ([Mainnet] Emergency Operator EOA)                |
| Governance proposer     | `cpi-multisig` `axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m`, needs 400,000 AXL per proposal |

| Chain       | Gateway proxy                                | Current implementation                       | Current auth module                          |
| ----------- | -------------------------------------------- | -------------------------------------------- | -------------------------------------------- |
| `celo`      | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `ethereum`  | `0x4F4495243837681061C4743b74B3eEdf548D56A5` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0xE3B83f79Fbf01B25659f8A814945aB82186A8AD0` |
| `avalanche` | `0x5029C0EFf6C34351a0CEc334542cDb22c7928f78` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0xE3B83f79Fbf01B25659f8A814945aB82186A8AD0` |
| `polygon`   | `0x6f015F16De9fC8791b234eF68D486d2bF203FBA8` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0xFcf8b865177c45A86a4977e518B44a1eD90191bd` |
| `binance`   | `0x304acf330bbE08d1e512eefaa92F6a57871fD895` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x242E937CE1B5894bf92c343e56eC63fAe522d627` |
| `arbitrum`  | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `kava`      | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `filecoin`  | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `optimism`  | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `linea`     | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `base`      | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x96eEE595f0ACD7d232a9B1dE4FCc2a4b2F6b0f3a` |
| `mantle`    | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0x4EC83aE13097EF1Fb1dC9fC98592bf607A22A55D` |
| `scroll`    | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x99B5FA03a5ea4315725c43346e55a6A6fbd94098` | `0xdC46f07661B673Fc262f61FC5b05B10A58a3b7fE` |
| `immutable` | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0x05823c334150a48ACD5D325fBA16147c21bA3653` | `0x1a920B29eBD437074225cAeE44f78FC700B27a5d` |
| `fraxtal`   | `0xe432150cce91c13a887f7D836923d5597adD8E31` | `0xc1712652326E87D193Ac11910934085FF45C2F48` | `0x1a920B29eBD437074225cAeE44f78FC700B27a5d` |

Out of scope:

- `fantom`: still active, but on-chain `governance()` is the old `InterchainGovernance`
  (`0xfDF36A30070ea0241d69052ea85ff44Ad0476a66`, 7-day timelock), which the governance chain rename skipped. Its
  `governanceChain` is still `Axelarnet`, while Axelar now sends governance calls from `axelar`, so the proposals in
  this release can't reach it. Upgrading fantom first needs its governance moved to the ASG.
- `moonbeam`, `centrifuge`: deactivated on Axelar, so they get no batches or operator rotations.
- `blast`: deactivated on Axelar without an upgrade, see the
  [Blast deactivation](../axelar/2026-10-Blast-Deactivate-Chain.md).

All in-scope chains use the same `ConstAddressDeployer` (`0x98B2920D…`) and `Create3Deployer` (`0x6513Aedb…`), so with
deployer `0xB1209B08…` every chain gets the same auth and implementation address. Both addresses are still free on all 15
chains. The implementation codehash differs per chain because of its `tokenDeployer` immutable; Step 2 records it.

| Contract             | Method    | Salt                                | Predicted address (deployer `0xB1209B08…`)   |
| -------------------- | --------- | ----------------------------------- | -------------------------------------------- |
| `AxelarAuthWeighted` | `create2` | `AxelarAuthWeighted v6.5.0 mainnet` | `0x3d3C73E91B1b0C243f1437F2d8363b1D5B093BD0` |
| `AxelarGateway` impl | `create3` | `AxelarGateway v6.5`                | `0x16C22cbDD77350b1AC88AdDA344F691c12f41960` |

## Pre-requisites

1. Update npm dependencies. The commands below need `governance.js relay` and `--title`/`--description`
   (feat/governance-relay-proposal, feat/governance-proposal-title) and the reuse-proxy config fix
   (fix/consensus-gateway-reuse-proxy-config).

```bash
npm ci && npm run build
```

The proposals take their deposit from the config. Expedited proposals use `govProposalExpeditedDepositAmount`, which
must match the chain's `expedited_min_deposit` of 400,000 AXL; a proposal with too low a deposit stays in the deposit
period instead of going to a vote:

```fish
jq -r .axelar.govProposalExpeditedDepositAmount axelar-chains-config/info/mainnet.json    # 400000000000
```

2. Set the shell variables (fish). `PRIVATE_KEY` is the deployer `0xB1209B08…`, and `MNEMONIC` is the governance
   proposer (`cpi-multisig`). Steps 5 and 7 execute with the ASG operator key instead. Don't name the chain list
   `CHAIN`: `cast` reads an exported `CHAIN` as its `--chain` option.

    `filecoin` is not in `$CHAINS`: its config sets `gasLimit` 500,000,000 without fees, so ethers' default 1.5 gwei
    priority fee makes every filecoin transaction reserve about 0.75 FIL up front. Its transactions run separately with
    explicit fees (`$FILECOIN_GAS`), because `--gasOptions` applies to every chain in `-n`. The proposals and the
    read-only checks include it through `$CHAINS,filecoin`.

```fish
set -e CHAIN
read -sx PRIVATE_KEY    # the deployer, 0xB1209B08…
read -sx MNEMONIC       # the governance proposer, cpi-multisig
set -x ENV mainnet
set -x CHAINS celo,ethereum,avalanche,polygon,binance,arbitrum,kava,optimism,linea,base,mantle,scroll,immutable,fraxtal

set ASG 0x7Acbae6CBa67d78AAf69e47000884aE00F9B2525
set OPERATOR 0xc8a8399c3D1207f4e109673be7047604737c1D56
set DEPLOYER 0xB1209B087e544785138784A50933d163A9087a7D
set AUTH_SALT "AxelarAuthWeighted v6.5.0 mainnet"
set IMPL_SALT "AxelarGateway v6.5"
set AUTH 0x3d3C73E91B1b0C243f1437F2d8363b1D5B093BD0 # predicted, confirmed in Step 1
set IMPL 0x16C22cbDD77350b1AC88AdDA344F691c12f41960 # predicted, confirmed in Step 2
set PAUSER 0x72164D4448fe6cfA472946feDC71e83b4628B1aF
set PAUSER_CALLDATA 0x4421ea2100000000000000000000000072164d4448fe6cfa472946fedc71e83b4628b1af # transferPauser($PAUSER)
set FILECOIN_GAS '{"gasLimit":500000000,"maxFeePerGas":2000000,"maxPriorityFeePerGas":1000000}'

# cfg <chain> <field>: a field of that chain in the config, e.g. cfg kava .rpc
function cfg
    jq -r ".chains[\"$argv[1]\"]$argv[2]" axelar-chains-config/info/$ENV.json
end
```

3. Fix the base RPC. The config's `https://developer-access-mainnet.base.org` does not answer:

```fish
set -l f axelar-chains-config/info/$ENV.json
jq '.chains.base.rpc = "https://mainnet.base.org"' $f >$f.tmp; and mv $f.tmp $f
```

4. Fund the deployer and the operator, then check the balances, that every gateway is governed by the ASG, and that its
   mint limiter is the config `Multisig` (otherwise Step 3 would change it):

```fish
for c in (string split , $CHAINS,filecoin)
    set rpc (cfg $c .rpc)
    set gw (cfg $c .contracts.AxelarGateway.address)
    echo "$c deployer="(cast balance $DEPLOYER -e --rpc-url $rpc)" operator="(cast balance $OPERATOR -e --rpc-url $rpc)" governance="(cast call $gw "governance()(address)" --rpc-url $rpc)" mintLimiter="(cast call $gw "mintLimiter()(address)" --rpc-url $rpc)" multisig="(cfg $c .contracts.Multisig.address)
end
curl -s https://axelar-lcd.qubelabs.io/cosmos/bank/v1beta1/balances/axelar14vps3ev03zyp2wmj89etx8rrxdxyltfy4rzl5m/by_denom?denom=uaxl
```

On 2026-11-09 the deployer held nothing on any chain. The operator was funded everywhere, but too little on base,
optimism, linea and scroll (see below). The deployer sends about 20 transactions per chain: the auth deploy (about 1.08M
gas), the implementation deploy (about 4.41M), 15 seeds (about 165k each with mainnet's 35 signers), a handoff and two
relays, about 8.3M gas in total. The operator sends two (about 65k and 165k gas).

The scripts use ethers v5, which pays a 1.5 gwei priority fee unless the config sets a gas price. On the OP Stack chains
and scroll that fee is far above the base fee and is most of the cost. A wallet must also hold gas limit × max fee for
each transaction up front: optimism, linea, scroll and fraxtal set a fixed 8M `gasLimit`, so every transaction there
reserves about 0.008-0.012 ETH, even the operator's. Suggested amounts (about twice the cost at 2026-11-09 prices,
and never below the up-front reservation; linea's gas price is volatile):

| Chain                         | Deployer    | Operator    |
| ----------------------------- | ----------- | ----------- |
| `ethereum`                    | 0.1 ETH     | 0.01 ETH    |
| `optimism`, `linea`, `scroll` | 0.03 ETH    | 0.02 ETH    |
| `fraxtal`                     | 0.03 frxETH | 0.02 frxETH |
| `base`                        | 0.03 ETH    | 0.002 ETH   |
| `arbitrum`                    | 0.01 ETH    | 0.002 ETH   |
| `celo`                        | 5 CELO      | 1 CELO      |
| `avalanche`                   | 0.2 AVAX    | 0.05 AVAX   |
| `polygon`                     | 10 POL      | 2 POL       |
| `binance`                     | 0.01 BNB    | 0.005 BNB   |
| `kava`                        | 0.1 KAVA    | 0.1 KAVA    |
| `filecoin`                    | 0.1 FIL     | 0.1 FIL     |
| `mantle`                      | 2 MNT       | 0.5 MNT     |
| `immutable`                   | 1 IMX       | 0.1 IMX     |

The proposer `cpi-multisig` holds about 1,000,995 AXL (2026-11-09), enough for both 400,000 AXL deposits at once.

## Announcement

Announce the upgrade well before submitting the Step 3 proposal, so integrators have time to react: the expedited vote
takes only 1 day. The gateway addresses and the `callContract` / `callContractWithToken` / `execute` interfaces stay the
same, and validators and relayers don't need to do anything. What changes for integrators:

- `sendToken` is removed. After the upgrade, calling it on a consensus gateway reverts, and `TokenSent` is no longer
  emitted. Use `callContractWithToken` or ITS instead. (In a 10,000-block sample on the eight largest chains on
  2026-10-18, no `TokenSent` was emitted.)
- A new pauser role (the Emergency Operator EOA) can pause the gateway. While paused, `callContract` and
  `callContractWithToken` revert.

Draft:

> **Axelar consensus gateway upgrade to v6.5.0 (mainnet)**
>
> Axelar governance will upgrade the `AxelarGateway` on celo, ethereum, avalanche, polygon, binance, arbitrum, kava,
> filecoin, optimism, linea, base, mantle, scroll, immutable and fraxtal to cgp-solidity v6.5.0. The governance
> proposal is expected on <date>, followed by a 1-day vote; each chain is upgraded shortly after the vote passes.
>
> - Gateway addresses and the `callContract`, `callContractWithToken` and `execute` interfaces do not change.
> - **`sendToken` is removed** and will revert after the upgrade. Contracts that still call `sendToken` should switch to
>   `callContractWithToken` or the Interchain Token Service.
> - The gateway gains an emergency pause controlled by a dedicated pauser. While paused, outgoing calls revert.
> - No action is needed from validators or relayers.

## Deployment Steps

### Steps 1–2: Deploy the auth and the implementation (deployer key)

Ethereum mainnet still charges about 200 gas per byte of deployed code. Sepolia's 2026-10-06 fork raised that to about
1,542, and if Ethereum mainnet adopts it the implementation deploy needs about 31M gas (see
[Fix ethereum-sepolia](./2026-10-Consensus-Gateway-Upgrade-v6.5.0.md#fix-ethereum-sepolia)). Check right before Step 2.
The difference between the two estimates is the cost of 9,000 bytes, so it should print about 200:

```fish
set rpc (cfg ethereum .rpc)
math "($(cast estimate --rpc-url $rpc --create 0x6127106000f3) - $(cast estimate --rpc-url $rpc --create 0x6103e86000f3)) / 9000"
```

If it prints about 1,542, add `--gasOptions '{"gasLimit":34000000}'` to ethereum's implementation deploy and run that
chain on its own.

```fish
npx ts-node evm/consensus-auth.js predict -e $ENV -n $CHAINS,filecoin -s $AUTH_SALT
npx ts-node evm/consensus-auth.js deploy  -e $ENV -n $CHAINS -s $AUTH_SALT
npx ts-node evm/consensus-auth.js deploy  -e $ENV -n filecoin -s $AUTH_SALT --gasOptions $FILECOIN_GAS

npx ts-node evm/deploy-consensus-gateway.js -e $ENV -n $CHAINS \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT
npx ts-node evm/deploy-consensus-gateway.js -e $ENV -n filecoin \
  --authModule $AUTH --reuseProxy --reuseHelpers \
  --deployMethod create3 --salt $IMPL_SALT --gasOptions $FILECOIN_GAS
```

- `predict` must print `$AUTH` and `Already deployed: no` on every chain.
- The "Contract already deployed at predicted address" warning shows each chain's own gateway. Check the address against
  the table above and answer `y`.
- `deploy-consensus-gateway.js` reported success on ethereum-sepolia although the deploy reverted. Check every chain
  on-chain before Step 3:

```fish
for c in (string split , $CHAINS,filecoin)
    set rpc (cfg $c .rpc)
    echo "$c size="(cast codesize $IMPL --rpc-url $rpc)" codehash="(test (cast keccak (cast code $IMPL --rpc-url $rpc)) = (cfg $c .contracts.AxelarGateway.implementationCodehash); and echo ok; or echo MISMATCH)" impl.auth="(cast call $IMPL "authModule()(address)" --rpc-url $rpc)" pending="(cfg $c .contracts.AxelarGateway.pendingAuthModule)" auth.owner="(cast call $AUTH "owner()(address)" --rpc-url $rpc)" auth.epoch="(cast call $AUTH "currentEpoch()(uint256)" --rpc-url $rpc)
end
```

Every chain should show size 18717, `codehash=ok`, `impl.auth` and `pending` equal to `$AUTH`, owner `$DEPLOYER` and
epoch 0.

### Step 3: Schedule the upgrade and relay

```fish
npx ts-node evm/governance.js schedule-operator upgrade 0 -e $ENV -n $CHAINS,filecoin \
  --targetContractName AxelarGateway --implementation $IMPL \
  --title "Upgrade consensus gateways to v6.5.0 ($CHAINS,filecoin)" \
  --description "Upgrades the AxelarGateway on $CHAINS,filecoin to $IMPL, bound to the new AxelarAuthWeighted $AUTH"
```

- For every chain, `Setup Params for upgrading` must be `0x` and the target must be that chain's gateway. If any chain
  shows something else, answer `n`.
- The "less than the minimum eta" warning is expected: the ASG ignores the activation time of an operator proposal.
- Before confirming the submission, check the encoded message: `sender` is the governance module
  `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj`, every `destination_address` is `$ASG`, and every payload starts with
  type `2` (`ApproveOperator`).

Once the proposal has passed (1-day expedited voting, or 3 days if it was converted to standard), relay it. If `relay`
reports that a call is not approved on the gateway yet, that chain's batch hasn't arrived; wait and retry.

```fish
set PROPOSAL_ID <proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n $CHAINS
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n filecoin --gasOptions $FILECOIN_GAS

for c in (string split , $CHAINS,filecoin)
    set calldata (cast calldata "upgrade(address,bytes32,bytes)" (cfg $c .contracts.AxelarGateway.implementation) (cfg $c .contracts.AxelarGateway.implementationCodehash) 0x)
    echo "$c approved="(cast call $ASG "isOperatorProposalApproved(address,bytes,uint256)(bool)" (cfg $c .contracts.AxelarGateway.address) $calldata 0 --rpc-url (cfg $c .rpc))
end
```

### Step 4: Seed the auth and verify (deployer key)

`prev_key_ids` reads a chain's most recent key rotations from Axelar's `KeyRotated` events and prints the 14 key IDs
before the current one, oldest first. `seed` adds the current key itself, so the auth ends at epoch 15, matching the
last 15 epochs of the live auth.

```fish
# prev_key_ids <chain>: the 14 key IDs before the chain's current one, oldest first
function prev_key_ids --argument-names chain
    set -l rpc (jq -r .axelar.rpc axelar-chains-config/info/$ENV.json)
    set -l lcd (jq -r .axelar.lcd axelar-chains-config/info/$ENV.json)
    set -l id (cfg $chain .axelarId)
    set -l current (curl -s $lcd/axelar/multisig/v1beta1/key_id/$id | jq -r .key_id)
    set -l query (string escape --style=url "\"axelar.multisig.v1beta1.KeyRotated.chain='\\\"$id\\\"'\"")
    set -l heights (curl -s "$rpc/block_search?query=$query&per_page=16&order_by=%22desc%22" | jq -r '.result.blocks[].block.header.height')
    set -l keys
    for h in $heights
        set -a keys (curl -s "$rpc/block_results?height=$h" | jq -r --arg c "\"$id\"" '.result.finalize_block_events[] | select(.type == "axelar.multisig.v1beta1.KeyRotated") | .attributes | from_entries | select(.chain == $c) | .key_id | fromjson')
    end
    set keys (string match -v -- $current $keys)[1..14]
    string join , $keys[-1..1]
end
```

Seed each chain right before its execute (Step 5). Check that `prev_key_ids` prints 14 key IDs, `evm-<chain>-<height>`
in increasing order, and that `seed` reports 15 sets:

```fish
for c in (string split , $CHAINS)
    npx ts-node evm/consensus-auth.js seed -e $ENV -n $c --authModule $AUTH --copyEpochs 0 --prevKeyIDs (prev_key_ids $c)
end
npx ts-node evm/consensus-auth.js seed -e $ENV -n filecoin --authModule $AUTH --copyEpochs 0 --prevKeyIDs (prev_key_ids filecoin) --gasOptions $FILECOIN_GAS

npx ts-node evm/consensus-auth.js verify -e $ENV -n $CHAINS,filecoin --authModule $AUTH
```

- `verify` warns that the deployer still owns the auth. That is expected until Step 6.
- `verify` refuses to run on a chain while a key rotation is in progress. Wait for it to land and re-run.
- If a rotation lands between `prev_key_ids` and `seed`, the newest key is missing. Re-run `seed` for that chain; it
  skips the sets that are already registered.

### Step 5: Execute the upgrade (operator key)

The readiness guard checks that the auth is still held by the seeder, so pass the deployer with `--authSeeder`. Check
that the script prints `Wallet address: 0xc8a8399c3D1207f4e109673be7047604737c1D56`; with any other key the transaction
reverts.

```fish
read -sx PRIVATE_KEY    # the ASG operator key, 0xc8a8399c…
for c in (string split , $CHAINS)
    set calldata (cast calldata "upgrade(address,bytes32,bytes)" (cfg $c .contracts.AxelarGateway.implementation) (cfg $c .contracts.AxelarGateway.implementationCodehash) 0x)
    npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $c \
      --target (cfg $c .contracts.AxelarGateway.address) --calldata $calldata --authSeeder $DEPLOYER
end
set calldata (cast calldata "upgrade(address,bytes32,bytes)" (cfg filecoin .contracts.AxelarGateway.implementation) (cfg filecoin .contracts.AxelarGateway.implementationCodehash) 0x)
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n filecoin \
  --target (cfg filecoin .contracts.AxelarGateway.address) --calldata $calldata --authSeeder $DEPLOYER --gasOptions $FILECOIN_GAS
```

`execute-operator-proposal` checks that the proposal is approved and runs the auth-readiness guard first, and aborts if
either fails. Do not bypass it. On success, config sets `AxelarGateway.authModule = $AUTH` and removes
`pendingAuthModule` for that chain.

### Step 6: Hand auth ownership to the proxy (deployer key)

Run this right after Step 5. Until the handoff, rotations don't register in the new auth.

```fish
read -sx PRIVATE_KEY    # the deployer, 0xB1209B08…
npx ts-node evm/consensus-auth.js handoff -e $ENV -n $CHAINS --authModule $AUTH
npx ts-node evm/consensus-auth.js handoff -e $ENV -n filecoin --authModule $AUTH --gasOptions $FILECOIN_GAS
npx ts-node evm/consensus-auth.js verify  -e $ENV -n $CHAINS,filecoin --authModule $AUTH
```

### Step 7: Set the pauser

Run it after the upgrade has executed: `transferPauser` only exists on the v6.5.0 implementation.
`--targetContractName AxelarGateway` takes each chain's own gateway as the target.

```fish
npx ts-node evm/governance.js schedule-operator raw 0 -e $ENV -n $CHAINS,filecoin \
  --targetContractName AxelarGateway --calldata $PAUSER_CALLDATA \
  --title "Set consensus gateway pauser ($CHAINS,filecoin)" \
  --description "Calls transferPauser($PAUSER) on the AxelarGateway on $CHAINS,filecoin"
```

Once it has passed, relay it, then execute with the operator key and check the pauser:

```fish
set PROPOSAL_ID <pauser proposal id>
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n $CHAINS
npx ts-node evm/governance.js relay $PROPOSAL_ID -e $ENV -n filecoin --gasOptions $FILECOIN_GAS

read -sx PRIVATE_KEY    # the ASG operator key, 0xc8a8399c…
for c in (string split , $CHAINS)
    npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n $c \
      --target (cfg $c .contracts.AxelarGateway.address) --calldata $PAUSER_CALLDATA
end
npx ts-node evm/governance.js execute-operator-proposal -e $ENV -n filecoin \
  --target (cfg filecoin .contracts.AxelarGateway.address) --calldata $PAUSER_CALLDATA --gasOptions $FILECOIN_GAS

npx ts-node evm/gateway.js -e $ENV -n $CHAINS,filecoin --action pauser
```

### Step 8: Commit the config

Commit `axelar-chains-config/info/mainnet.json`. For every chain it should change `implementation`,
`implementationCodehash`, `authModule` and `salt`, and record `governance` and `mintLimiter`, with no
`pendingAuthModule` left. Leave out the base RPC change, or commit it on its own.

## Failure handling

| Situation                               | Action                                                                                                                                                       |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Proposal rejected                       | Re-schedule (Step 3). `AUTH` and `IMPL` can be reused.                                                                                                       |
| Proposal not relayed                    | `relay` again; it reports when a chain's batch hasn't reached the gateway yet.                                                                               |
| Proposal approved but wrong             | Do not execute. Cancel the approval with `cancel-operator`, then fix and re-schedule.                                                                        |
| `execute-operator-proposal` reverts     | Check that `PRIVATE_KEY` is the ASG operator key and that the proposal is relayed.                                                                           |
| `verify` fails before execute           | Re-run `seed`, then `verify`. Do not execute until it passes.                                                                                                |
| Readiness guard aborts execution        | Re-seed and retry. Do not bypass the guard.                                                                                                                  |
| Execution slipped and the seed aged out | Re-run `seed` (repeatable), then `verify`.                                                                                                                   |
| Proposer lacks the 400,000 AXL deposit  | Wait for the previous proposal's deposit to be refunded, or top up `cpi-multisig`.                                                                           |
| `handoff` fails                         | The gateway keeps working on the seeded sets. Re-run `handoff` within 16 rotations (~6 weeks).                                                               |
| A chain's deploy reverted               | Find the cause before retrying, as on [ethereum-sepolia](./2026-10-Consensus-Gateway-Upgrade-v6.5.0.md#fix-ethereum-sepolia). The other chains can continue. |

## Checklist

Run this for each chain after Step 7. Record the result in the status table at the top, and note any item that is
still pending there.

- [ ] Gateway `implementation()` returns `$IMPL`.
- [ ] Gateway `authModule()` returns `$AUTH`.
- [ ] `$AUTH` `owner()` is the gateway proxy, and `consensus-auth verify` passes.
- [ ] Gateway `pauser()` returns `$PAUSER`, and `paused()` returns `false`.
- [ ] A GMP call with the chain as destination is approved and executed.
- [ ] A GMP call with the chain as source is relayed.
- [ ] After the next rotation (about every 3 days), `$AUTH` `currentEpoch()` has advanced past 15.
