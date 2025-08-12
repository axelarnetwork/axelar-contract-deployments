# SolanaMultisigProver v0.2.0

|                | **Owner** |
| -------------- | --------- |
| **Created By** | @eigerco  |
| **Deployment** | @eigerco  |

| **Network**          | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Deployed              | 2025-08-12 |
| **Stagenet**         | -                     | TBD        |
| **Testnet**          | -                     | TBD        |
| **Mainnet**          | -                     | TBD        |

- [Release](https://github.com/eigerco/axelar-amplifier/releases/tag/solana-multisig-prover-v0.2.0)
- SolanaMultisigProver checksum: `f6c983a2d6d9de92a5ecafe02ed76f414af104cd9734c40c25506b83a80ac34b`


## Background

No changes in the Solana cosmwasm side. This is an upgrade mainly for testing purposes.

## Deployment

- This rollout upgrades SolanaMultisigProver from `v0.1.0` to `v0.2.0`
- The migrate step will just update the code. No state will be modified.

1. Create `.env`.

| Network              | `INIT_ADDRESSES`                                                                                                                            | `RUN_AS_ACCOUNT`                                |
| -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| **Devnet-amplifier** | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9`                                               | `axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9` |
| **Stagenet**         | `axelar1pumrull7z8y5kc9q4azfrmcaxd8w0779kg6anm,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12qvsvse32cjyw60ztysd3v655aj5urqeup82ky` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Testnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar12f2qn005d4vl03ssjq07quz6cja72w5ukuchv7` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |
| **Mainnet**          | `axelar1uk66drc8t9hwnddnejjp92t22plup0xd036uc2,axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj,axelar1nctnr9x0qexemeld5w7w752rmqdsqqv92dw9am` | `axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj` |

| Network              | `DEPOSIT_VALUE` |
| -------------------- | --------------- |
| **Devnet-amplifier** | `100000000`     |
| **Stagenet**         | `100000000`     |
| **Testnet**          | `2000000000`    |
| **Mainnet**          | `2000000000`    |

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

1. Clone and checkout the correct branch:
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

3. Store `SolanaMultisigProver` contract.

```bash
ts-node cosmwasm/submit-proposal.js store \
  -c SolanaMultisigProver \
  -t "Upload SolanaMultisigProver contract v0.2.0" \
  -d "Upload SolanaMultisigProver contract v0.2.0" \
  -a "$ARTIFACT_PATH/solana_multisig_prover.wasm" \
  --deposit $DEPOSIT_VALUE \
  --instantiateAddresses $INIT_ADDRESSES
```

6. Migrate `SolanaMultisigProver` contract.

```bash
ts-node cosmwasm/submit-proposal.js migrate \
  -c SolanaMultisigProver \
  -t "Migrate SolanaMultisigProver to v0.2.0" \
  -d "Migrate SolanaMultisigProver to v0.2.0" \
  --msg '{}' \
  --fetchCodeId \
  --deposit $DEPOSIT_VALUE
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
