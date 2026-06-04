# EVM Amplifier — Pausable Gateway + ASG rollout (testnet)

Upgrade each EVM amplifier gateway to the pausable `AxelarAmplifierGateway`, deploy
`AxelarServiceGovernance` (ASG), and transfer gateway ownership to the ASG.

SDK: `@axelar-network/axelar-gmp-sdk-solidity@6.2.0` (pausable; 6.1.0 is not). package.json pinned to it.

## Keys
- ASG deployer: `0x81e63eA8F64FEdB9858EB6E2176B431FBd10d1eC` — must deploy the ASG (create3, salt `v6.1.0`,
  canonical Create3Deployer `0x6513Aedb…` → shared `0xb55A09E9fde39755c39EF9A7911431Ba0c96332A`).
- Gateway owner (for `--upgrade` / `transferOwnership`): `0xF140b…` most chains, `0x49845e…` for monad-3/arc-8.
  Query `owner()` on-chain — the config `owner` field is stale.
- ASG operator: `0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e`, delay 300.

## Procedure
```bash
# 1. deploy impl (any funded key); pass the chain's live immutables
ts-node evm/deploy-amplifier-gateway.js -e testnet -n <chain> --reuseProxy --deployMethod create \
  --previousSignersRetention 15 --domainSeparator <live> --minimumRotationDelay 3600

# 2. upgrade proxy — EOA owner:
ts-node evm/deploy-amplifier-gateway.js -e testnet -n <chain> --upgrade
#    already ASG-owned: via governance instead:
axe propose testnet <chain> --target <gateway> --calldata <gateway.upgrade(impl,codehash,0x)> --relay

# 3. deploy ASG (deployer 0x81e63…); confirm predicted addr is 0xb55A09… before y
ts-node evm/deploy-contract.js -c AxelarServiceGovernance -m create3 -e testnet -n <chain> \
  --salt "v6.1.0" --args '{"minimumTimeDelay":300,"operator":"0x6e079CD1c6bBb72680DeDF2687d711AE9427eE8e"}'

# 4. transfer ownership (owner key)
ts-node evm/ownership.js -c AxelarGateway -e testnet -n <chain> --action transferOwnership --newOwner 0xb55A09…
```
Verify: `implementation()` = new impl, `owner()` = ASG, `paused()` = false, `epoch()` unchanged.
Optional gov-path check: `axe propose testnet <chain> --op unpause --relay` (no-op).

Commit only `implementation` / `implementationCodehash` / `owner` (=ASG) + the `AxelarServiceGovernance` block.
The deploy scripts also overwrite `deployer` and stringify `previousSignersRetention` (`"15"`) — revert those.
Never commit RPC URLs with API keys or `gasLimit`.

## Gotchas
- celo-sepolia (forno) / hedera (hashio) RPCs are eventually-consistent: the deploy can throw
  `Contract bytecode is empty` after a successful deploy. Verify on-chain, set impl+codehash in config by hand,
  then `--upgrade`.
- hyperliquid: deploys need big blocks — `evm/hyperliquid.js update-block-size big` per deployer, which needs a
  HyperCore account. Activate with `evm/hyperliquid.js usd-send <addr> <amt>` from `0xba76…` (or the faucet).
  Owner `--upgrade`/`transferOwnership` are small calls — no big blocks needed.
- monad-3 has no canonical Create3Deployer → ASG at `0x58d170fe3f97813B13442B1a924F9a8CCa2bB22f`, not `0xb55A09…`.

## Status
| Chain | pausable | ASG (owner) | gov tested | impl |
|---|---|---|---|---|
| berachain | ✅ | `0xb55A09…` | — | |
| plume | ✅ | `0xb55A09…` | ✅ | |
| flow | ✅ | `0xb55A09…` | — | |
| xrpl-evm | ✅ (via gov) | `0xb55A09…` | ✅ 604 | `0x278291…` |
| monad-3 | ✅ | `0x58d170fe…` | ✅ 607 | `0x3cfc5f38…` |
| arc-8 | ✅ | `0xb55A09…` | ✅ 605 | `0xc3Fd691d…` |
| celo-sepolia | ✅ | `0xb55A09…` | ✅ 606 | `0xDA3Ad9…` |
| hyperliquid | ✅ | `0xb55A09…` | ✅ 608 | `0x253Fd90c…` |
| hedera | in progress | — | — | |

### Deferred
- test-sepolia / test-avalanche: amplifier (gw `0x14213B104d…`, owner `0xba76…` — ours) but not in main config.
  Bootstrap from `2e420e49~1` (before PR #476 removed them), verify on-chain, separate PR.
- memento-demo: no RPC in config.
- monad (canonical `0xe432150c…`): no code there — use monad-3.
- sui / stellar / solana / xrpl: non-EVM, out of scope.
