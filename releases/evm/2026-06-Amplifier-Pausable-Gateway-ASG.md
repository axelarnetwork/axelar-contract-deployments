# EVM Amplifier — Pausable Gateway + Service Governance rollout

Roll the **pausable** `AxelarAmplifierGateway` and **`AxelarServiceGovernance`** (ASG) out to every EVM
amplifier chain, and put each gateway under governance (owner = ASG, like Berachain).

> WIP / incremental: this branch accumulates the per-chain config records + status as chains are rolled out.

## Prerequisites
- **`@axelar-network/axelar-gmp-sdk-solidity` `v6.2.0` published on npm** (the pausable gateway + ASG). The
  upgrades deploy the implementation from this version — **gated until it's on the registry**.
  (Berachain below was done from a local pre-release build; re-verify its `implementationCodehash` against
  the official `v6.2.0` bytecode before relying on it.)
- Per-chain gateway **owner key** (for `--upgrade` and `transferOwnership`); a funded deployer key.
- ASG `operator` = an emergency EOA/multisig.

## Per-chain procedure (as done on Berachain)
1. **Upgrade gateway → pausable**, reusing the proxy, matching the chain's live immutables
   (`minimumRotationDelay`, `domainSeparator`, `previousSignersRetention`) and deploy method/salt:
   ```bash
   ts-node evm/deploy-amplifier-gateway.js -e testnet -n <chain> --reuseProxy -m <create|create2|create3> \
     --minimumRotationDelay <live> --previousSignersRetention <live> [--domainSeparator <live>]
   ts-node evm/deploy-amplifier-gateway.js -e testnet -n <chain> --upgrade          # owner key
   ```
   Verify `paused()` is unchanged (all current gateways are unpaused; the upgrade defaults to unpaused).
2. **Deploy ASG if the chain doesn't have one** (`governanceChain=axelar`, `governanceAddress`=gov module,
   small `minimumTimeDelay`, `operator`=emergency EOA):
   ```bash
   ts-node evm/deploy-contract.js -c AxelarServiceGovernance -m create2 -e testnet -n <chain> \
     --args '{"minimumTimeDelay":<secs>,"operator":"<emergency EOA>"}'
   ```
3. **Transfer gateway ownership → ASG** (owner = ASG, operator stays an EOA):
   ```bash
   ts-node evm/ownership.js -c AxelarGateway -e testnet -n <chain> \
     --action transferOwnership --newOwner <ASG address>          # owner key
   ```
4. **Verify on-chain:** `implementation()` = new impl, `owner()` = ASG, `paused()` unchanged; ASG
   `governanceChain()/governanceAddress()/operator()`.

## Status — testnet EVM amplifier chains
| Chain | Upgraded to pausable | ASG deployed | owner = ASG | Notes |
|---|---|---|---|---|
| berachain | ✅ `0x39fD…9F49` | ✅ `0x848aEc…4b0D` | ✅ | done from pre-release build; re-verify codehash vs v6.2.0; currently **paused** |
| flow | ⬜ | ⬜ | ⬜ | |
| hedera | ⬜ | ⬜ | ⬜ | |
| xrpl-evm | ⬜ | ⬜ | ⬜ | |
| plume | ⬜ | ⬜ | ⬜ | |
| monad | ⬜ | ⬜ | ⬜ | |
| hyperliquid | ⬜ | ⬜ | ⬜ | |
| celo-sepolia | ⬜ | ⬜ | ⬜ | |
| arc | ⬜ | ⬜ | ⬜ | |
| memento-demo | ⬜ | ⬜ | ⬜ | (demo — confirm if in scope) |

Pause/unpause via governance once owned by ASG: see the operator fast-path / timelock proposal flow
(`evm/governance.js`). The gateway's own `operator` EOA can still emergency-pause directly.
