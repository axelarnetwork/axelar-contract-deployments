# Solana Program upgrade tracking doc <Date> (this is template)

This guide is for **upgrading Solana programs** as part of the GMP/ITS v1.0.0 initial release ([1](./2025-07-GMP-v1.0.0.md), [2](./2025-07-ITS-v1.0.0.md) respectively). It assumes that:

- Programs are already deployed with known program IDs.
- The upgrade authority keypair is available.
- You're upgrading using the same verifiable build process `solana-verify`.


## Program Upgrade Tracking

| Program     | Env                | From version | To version | From hash | To hash | ✅ Done |
| ----------- | ------------------ | ------------ | ---------- | --------- | ------- | ------ |
| Gateway     | `devnet-amplifier` |              |            |           |         |        |
| ITS         | `devnet-amplifier` |              |            |           |         |        |
| Gas Service | `devnet-amplifier` |              |            |           |         |        |
| Governance  | `devnet-amplifier` |              |            |           |         |        |
| Multicall   | `devnet-amplifier` |              |            |           |         |        |

Where `Env` can be:

* devnet-amplifier
* Stagenet
* Testnet
* Mainnet

## Prerequisites

1. **Build environment**

   ```bash
   export BASE_IMAGE="solanafoundation/solana-verifiable-build@sha256:979b09eef544de4502a92e28a724a8498a08e2fe506e8905b642e613760403d3"
   export ENV=<devnet-amplifier|stagenet|testnet|mainnet>
   export CHAIN_ID=<chain-id>
   ```

2. **Build the updated binaries**

In the [programs repository](https://github.com/eigerco/solana-axelar) root, build only the programs you need to upgrade.

   ```bash
solana-verify build --base-image $BASE_IMAGE --library-name <library_name> -- --no-default-features --features $ENV

   ```

   Where `library_name` can be:

   * axelar_solana_gateway
   * axelar_solana_its
   * axelar_solana_gas_service
   * axelar_solana_governance
   * axelar_solana_multicall

## Upgrade programs

1. **Declare environment variables**


   ```bash
export PROGRAM_BYTECODE_PATH="solana-axelar/target/deploy/<program_name>.so"
export PROGRAM_ID=<PROGRAM_ID>

export UPGRADE_AUTHORITY_KEYPAIR_PATH=<path/to/upgrade_authority_keypair.json>
export COMMIT_HASH=$(git -C solana-axelar rev-parse HEAD)
   ```

   **Note**: `PROGRAM_BYTECODE_PATH` and `PROGRAM_ID` needs to be updated for each program that is going to be deployed.

2. **Set solana CLI on the convenient cluster**

   ```bash
   solana config set --url <mainnet|devnet>
   ```
   note: We deploy all Axelar test environments in devnet
   

3. **Upgrade Programs**

There is a special CLI command that will get the program_id for you:

```bash
./solana/solana-axelar-cli upgrade --program <gateway|gas-service|governance|its> $PROGRAM_BYTECODE_PATH
```

## Verify

Verification is **only possible in mainnet**. If deploying for test environments you can skip this step.

```bash
solana-verify verify-from-repo --remote --base-image $BASE_IMAGE \
  --commit-hash $COMMIT_HASH \
  --program-id $PROGRAM_ID \
  https://github.com/eigerco/solana-axelar \
  -- --no-default-features --features $ENV
```

## Post-Upgrade Checklist

- [ ] Re-run `GMP` test transaction (see final section in original deployment docs ([1](./2025-07-GMP-v1.0.0.md), [2](./2025-07-ITS-v1.0.0.md)).
- [ ] Run the [e2e repository](https://github.com/eigerco/axelar-solana-e2e) pipeline.
