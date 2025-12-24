# ITS Hub Scripts

Scripts for managing Interchain Token Service (ITS) hub operations on consensus chains.

> **Note:** These scripts were developed for the P2P token migration to ITS Hub (see release docs in `releases/evm/2025-12-ITS-hub-*`). They remain available for ongoing operational use cases.

---

## Prerequisites

```bash
# Store ITS operator mnemonic in .env
MNEMONIC="[ITS operator mnemonic]"
ENV="[mainnet | testnet | stagenet | devnet-amplifier]"
```

---

## Commands

### Register P2P Token

Register a single token instance on ITS Hub:

```bash
ts-node cosmwasm/its.ts register-p2p-token \
  --chain <chain> \
  --tokenId <tokenId> \
  --originChain <originChain> \
  --decimals <decimals> \
  --dryRun  # optional
```

### Check Token Registration

Query which chains a token is registered on:

```bash
ts-node cosmwasm/its.ts registered-chains-by-token <tokenId>
```

### Align Token Supply

Sync the ITS Hub supply state with actual EVM chain supply:

```bash
ts-node cosmwasm/its.ts align-token-supply \
  --tokenId <tokenId> \
  --chain <chain> \
  --tokenAddress <tokenAddress> \
  --dryRun  # optional
```

**Supply tracking logic:**
- Origin chain tokens → always `untracked`
- Non-origin chain tokens → `tracked` if `isMinter(address(0))` returns true
- Calculates supply delta and issues `increase_supply` or `decrease_supply` calls

---

## Batch Operations

> **Note:** These batch scripts are not on `main`. See [PR #1136](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1136) for the implementation.

For processing multiple tokens from config files:

| Command | Description |
|---------|-------------|
| `register-tokens` | Register all tokens from config |
| `align-token-supply` | Align supply for all tokens |

```bash
ts-node cosmwasm/migrate/register-p2p-tokens.ts <command> \
  -n <chains...>       # optional: filter by chains
  --tokenIds <ids...>  # optional: filter by token IDs
  --dryRun             # optional
```

Config file location: `axelar-chains-config/info/tokens-p2p/tokens-${env}.json`

---

## Token Discovery

> **Note:** This script is not on `main`. See [PR #1136](https://github.com/axelarnetwork/axelar-contract-deployments/pull/1136) for the implementation.

Index P2P tokens deployed across consensus chains:

```bash
ts-node cosmwasm/migrate/get-p2p-tokens.ts -e <env>
```

Scans `TokenManagerDeployed` events, retrieves token metadata, and writes to config file.

---

## Related Queries

```bash
ts-node cosmwasm/query.js token-config <tokenId>
ts-node cosmwasm/query.js token-instance <chainName> <tokenId>
ts-node cosmwasm/query.js its-chain-config <chainName>
```
