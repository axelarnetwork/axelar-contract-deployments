# Solana Program Upgrade Doc [current date]

This guide is for **upgrading Solana programs** as part of the GMP/ITS v1.0.0 initial release ([1](./2025-09-GMP-v1.0.0.md), [2](./2025-09-ITS-v1.0.0.md) respectively). It assumes that:

- Programs are already deployed with known PDAs.
- The upgrade authority keypair is available.
- You're upgrading using the same verifiable build process `solana-verify`.

## Program Upgrade Tracking

| **Axelar Env**       | **Program**     | **From Version** | **To Version** | **From Hash** | **To Hash** | **Done?** |
| -------------------- | --------------- | ---------------- | -------------- | ------------- | ----------- | --------- |
| **Devnet Amplifier** | Gateway         |                  |                |               |             |           |
| **Devnet Amplifier** | ITS             |                  |                |               |             |           |
| **Devnet Amplifier** | Gas Service     |                  |                |               |             |           |
| **Devnet Amplifier** | Governance      |                  |                |               |             |           |
| **Devnet Amplifier** | Multicall       |                  |                |               |             |           |

| **Axelar Env**       | **Program**     | **From Version** | **To Version** | **From Hash** | **To Hash** | **Done?** |
| -------------------- | --------------- | ---------------- | -------------- | ------------- | ----------- | --------- |
| **Stagenet**         | Gateway         |                  |                |               |             |           |
| **Stagenet**         | ITS             |                  |                |               |             |           |
| **Stagenet**         | Gas Service     |                  |                |               |             |           |
| **Stagenet**         | Governance      |                  |                |               |             |           |
| **Stagenet**         | Multicall       |                  |                |               |             |           |

| **Axelar Env**       | **Program**     | **From Version** | **To Version** | **From Hash** | **To Hash** | **Done?** |
| -------------------- | --------------- | ---------------- | -------------- | ------------- | ----------- | --------- |
| **Testnet**          | Gateway         |                  |                |               |             |           |
| **Testnet**          | ITS             |                  |                |               |             |           |
| **Testnet**          | Gas Service     |                  |                |               |             |           |
| **Testnet**          | Governance      |                  |                |               |             |           |
| **Testnet**          | Multicall       |                  |                |               |             |           |

| **Axelar Env**       | **Program**     | **From Version** | **To Version** | **From Hash** | **To Hash** | **Done?** |
| -------------------- | --------------- | ---------------- | -------------- | ------------- | ----------- | --------- |
| **Mainnet**          | Gateway         |                  |                |               |             |           |
| **Mainnet**          | ITS             |                  |                |               |             |           |
| **Mainnet**          | Gas Service     |                  |                |               |             |           |
| **Mainnet**          | Governance      |                  |                |               |             |           |
| **Mainnet**          | Multicall       |                  |                |               |             |           |

Note: Current deployed contract hashes can be obtained with the following sequence of commands. In example,
getting the current Solana devnet governance address:

1. We first calculate the buffer account address from the program address

    ```bash
    solana program show govmXi41LqLpRpKUd79wvAh9MmpoMzXk7gG4Sqmucx9
    ```

    Output:

    ```sh
    Program Id: govmXi41LqLpRpKUd79wvAh9MmpoMzXk7gG4Sqmucx9
    Owner: BPFLoaderUpgradeab1e11111111111111111111111
    ProgramData Address: Dx7fpgZQWpSi6RD1p1wXcrdm5a7dRVEWL6YSHNMtN2ZT
    Authority: upaFrJck9TeFUXW62r2dDJtBxcMa4ArVjQ49sJeGDVw
    Last Deployed In Slot: 395770157
    Data Length: 289256 (0x469e8) bytes
    Balance: 2.01442584 SOL
    ```

1. We calculate the hash of the bytecode with the obtained buffer account address from `1`

    ```bash
    solana-verify get-buffer-hash Dx7fpgZQWpSi6RD1p1wXcrdm5a7dRVEWL6YSHNMtN2ZT
    ```

    Output:

    ```sh
    ae907491891a48851a4e347d4a23a41ad73e8b2fec8664951ed76011b31ee9e1
    ```

## Prerequisites

1. **Build environment**

   ```bash
   BASE_IMAGE="solanafoundation/solana-verifiable-build@sha256:979b09eef544de4502a92e28a724a8498a08e2fe506e8905b642e613760403d3"
   COMMIT_HASH="<latest axelar-amplifier-solana commit hash>"
   ENV=<devnet-custom|devnet-amplifier|stagenet|testnet|mainnet>
   CLUSTER=<devnet|testnet|mainnet-beta>
   CHAIN=<solana-custom|solana>
   ```

1. **Build the updated binaries**

    In the [programs repository](https://github.com/axelarnetwork/axelar-amplifier-solana) root, build only the programs you need to upgrade.

   ```bash
    solana-verify build --base-image $BASE_IMAGE --library-name <library_name> -- --no-default-features --features $ENV
   ```

   Where `library_name` can be:

   - axelar_solana_gateway
   - axelar_solana_its
   - axelar_solana_gas_service
   - axelar_solana_governance
   - axelar_solana_multicall

## Upgrade programs

1. **Declare environment variables**

   ```bash
    PROGRAM_PDA=<program PDA>
    PROGRAM_PATH="axelar-amplifier-solana/target/deploy/<program_name>.so"

    UPGRADE_AUTHORITY_KEYPAIR_PATH="<path/to/upgrade_authority_keypair.json>"
   ```

   **Note**: `PROGRAM_PDA` and `PROGRAM_PATH` need to be updated for each program that will be upgraded.

1. **Set Solana CLI to the relevant cluster**

   ```bash
   solana config set --url $CLUSTER
   ```

1. **Upgrade Programs**

    There is a special CLI command that will get the program_id for you:

    ```bash
    solana/cli upgrade --program <gateway|gas-service|governance|its> $PROGRAM_PATH
    ```

## Verify

Verification is **only possible in mainnet**. If deploying for test environments you can skip this step.

```bash
solana-verify verify-from-repo --remote --base-image $BASE_IMAGE \
    --commit-hash $COMMIT_HASH \
    --program-id $PROGRAM_PDA \
    https://github.com/axelarnetwork/axelar-amplifier-solana \
    -- --no-default-features --features $ENV
```

## Post-Upgrade Checklist

- [ ] Re-run `GMP` test transaction (see final section in original deployment docs ([1](./2025-09-GMP-v1.0.0.md), [2](./2025-09-ITS-v1.0.0.md)).
- [ ] Run the [e2e repository](https://github.com/eigerco/axelar-solana-e2e) pipeline.
