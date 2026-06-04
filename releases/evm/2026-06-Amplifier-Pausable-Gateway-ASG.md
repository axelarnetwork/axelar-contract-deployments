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

## Status
| Chain | pausable | ASG (owner) | gov tested | impl |
|---|---|---|---|---|
| berachain | ✅ | `0xb55A09…` | ✅ | |
| plume | ✅ | `0xb55A09…` | ✅ | |
| flow | ✅ | `0xb55A09…` | ✅ | |
| xrpl-evm | ✅ (via gov) | `0xb55A09…` | ✅ 604 | `0x278291…` |
| monad-3 | ✅ | `0x58d170fe…` | ✅ 607 | `0x3cfc5f38…` |
| arc-8 | ✅ | `0xb55A09…` | ✅ 605 | `0xc3Fd691d…` |
| celo-sepolia | ✅ | `0xb55A09…` | ✅ 606 | `0xDA3Ad9…` |
| hyperliquid | ✅ | `0xb55A09…` | ✅ 608 | `0x253Fd90c…` |
| hedera | ✅ | `0xb55A09…` | ✅ 609 | `0xD66F74B2…` |
| test-sepolia | ✅ | `0xb55A09…` | pending | `0xC4Cd8FCa…` |
| test-avalanche | ✅ | `0xb55A09…` | pending | `0x435dE0d2…` |

### Out of scope
- sui / stellar / solana / xrpl — non-EVM amplifier chains; this rollout is EVM-only.
