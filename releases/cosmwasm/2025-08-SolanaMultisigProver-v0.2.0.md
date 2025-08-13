# SolanaMultisigProver v0.2.0

|                | **Owner** |
| -------------- | --------- |
| **Created By** | @eigerco  |
| **Deployment** | @eigerco @kulikthebird  |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-08-12 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

- [Release](https://github.com/eigerco/axelar-amplifier/releases/tag/solana-multisig-prover-v0.2.0)
- SolanaMultisigProver checksum: `09a749a0bcd854fb64a2a5533cc6b0d5624bbd746c94e66e1f1ddbe32d495fb6`


## Background

No changes in the Solana cosmwasm side. This is an upgrade mainly for testing purposes.

## Deployment

- This rollout upgrades SolanaMultisigProver from `v0.1.0` to `v0.2.0`
- The migrate step will just update the code. No state will be modified.

1. Create `.env`.


```bash
MNEMONIC=xyz
ENV=abc
CHAIN=solana-2
ARTIFACT_PATH=
INIT_ADDRESSES=
RUN_AS_ACCOUNT=
DEPOSIT_VALUE=
MULTISIG_PROVER=$(cat "./axelar-chains-config/info/${ENV}.json" | jq ".axelar.contracts.MultisigProver[\"$CHAIN\"].address" | tr -d '"')
```

```bash
source .env
```

2. Clone and checkout the correct branch:
```bash
git clone --recurse-submodules https://github.com/eigerco/axelar-amplifier.git
cd axelar-amplifier
git checkout solana-cosmwasm
```

2. Build the contracts and copy artifacts:
```bash
docker run --rm -v "$(pwd)":/code \
      --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
      --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
      cosmwasm/optimizer:0.16.1
```
That would create the `artifacts` folder with all the compiled contracts, plus the checksums.

4. Store `SolanaMultisigProver` contract.

```bash
ts-node cosmwasm/submit-proposal.js store \
  -c MultisigProver \
  -t "Upload SolanaMultisigProver contract v0.2.0" \
  -d "Upload SolanaMultisigProver contract v0.2.0" \
  -a "$ARTIFACT_PATH"
```

6. Migrate `SolanaMultisigProver` contract.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c MultisigProver \
  -t "Migrate SolanaMultisigProver to v0.2.0" \
  -d "Migrate SolanaMultisigProver to v0.2.0" \
  --msg '{}' \
  --fetchCodeId
```

## Checklist

Verify `SolanaMultisigProver` contract version:

```bash
axelard query wasm contract-state raw $MULTISIG_PROVER 636F6E74726163745F696E666F -o json | jq -r '.data' | base64 -d
```

Expected output

```bash
{"contract":"solana-multisig-prover","version":"0.2.0"}
```
Ensure all systems are running by following the checklist of the following documents:
* Follow the [Solana GMP checklist](../solana/2025-07-GMP-v1.0.0.md)
* Follow the [Solana ITS checklist](../solana/2025-07-ITS-v1.0.0.md)
