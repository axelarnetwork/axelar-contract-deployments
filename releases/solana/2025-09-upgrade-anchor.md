# Solana GMP Upgrade (anchor to anchor)

## Network Status

|                | **Owner**    |
| -------------- | ------------ |
| **Created By** | @nbayindirli |
| **Deployment** | @nbayindirli |

| **Axelar Env**       | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Pending               | TBD        |
| **Stagenet**         | Pending               | TBD        |
| **Testnet**          | Pending               | TBD        |
| **Mainnet**          | Pending               | TBD        |

## Background

This is the Solana GMP upgrade doc for anchor TO anchor programs.

## Deployment

### Deployment Prerequisites

- A GMP/ITS version should already be deployed, per another release. Examples:
  - [2025-09-GMP-anchor](./2025-09-GMP-anchor.md)
  - [2025-09-ITS-anchor](./2025-09-ITS-anchor.md)

1. Ensure all `Deployment Prerequisites` in [2025-09-GMP-v1-upstream](./2025-09-GMP-v1-upstream.md) are already met.

### Deployment Steps

1. Ensure the environment variables set in `Deployment Setup` in [2025-09-GMP-v1-upstream](./2025-09-GMP-v1-upstream.md) are still sourced.

1. Clone the [`axelar-amplifier-solana`](https://github.com/axelarnetwork/axelar-amplifier-solana) repo.

1. Ensure the branch you would like to upgrade to is checked out and cd to the new repo.

1. Compile the Solana programs:

    ```sh
    # Go to the solana directory within the cloned repo
    cd axelar-amplifier-solana

    # Compile the Solana programs
    solana-verify build --base-image $BASE_IMAGE --library-name solana_axelar_gas_service
    solana-verify build --base-image $BASE_IMAGE --library-name solana_axelar_gateway
    solana-verify build --base-image $BASE_IMAGE --library-name solana_axelar_governance
    solana-verify build --base-image $BASE_IMAGE --library-name solana_axelar_multicall
    solana-verify build --base-image $BASE_IMAGE --library-name solana_axelar_memo_program

    # Go back
    cd ..
    ```

1. Reassign the follow environment variables for the programs you are upgrading:

    ```sh
    GATEWAY_PROGRAM_KEYPAIR_PATH="<path/to/gateway_program_keypair.json>"
    GATEWAY_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/solana_axelar_gateway.so"
    GATEWAY_PDA="[gateway-pda]"

    GAS_SERVICE_PROGRAM_KEYPAIR_PATH="<path/to/gas_service_program_keypair.json>"
    GAS_SERVICE_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/solana_axelar_gas_service.so"
    GAS_SERVICE_PDA="[gas-service-pda]"

    GOVERNANCE_PROGRAM_KEYPAIR_PATH="<path/to/governance_program_keypair.json>"
    GOVERNANCE_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/solana_axelar_governance.so"
    GOVERNANCE_PDA="[governance-pda]"

    MULTICALL_PROGRAM_KEYPAIR_PATH="<path/to/multicall_program_keypair.json>"
    MULTICALL_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/solana_axelar_multicall.so"
    MULTICALL_PDA="[multicall-pda]"

    MEMO_PROGRAM_KEYPAIR_PATH="<path/to/memo_program_keypair.json>"
    MEMO_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/solana_axelar_memo_program.so"
    MEMO_PDA="[memo-pda]"
    ```

    ```bash
    source .env
    ```

1. Ensure Solana CLI is set to the relevant cluster:

   ```bash
   solana config set --url $CLUSTER
   ```

1. Upgrade the programs:

    ```sh
    anchor upgrade --provider.cluster $CLUSTER -p $PROGRAM_PDA $PROGRAM_PATH
    ```

1. Verify the programs:

    > [!NOTE]
    > Verification is **only possible in mainnet**. If deploying for test environments you can skip this step.

    ```bash
    anchor verify -p [axelar_solana_program_name] --provider.cluster $CLUSTER $(solana address -k $PROGRAM_KEYPAIR_PATH) -- --no-default-features --features $ENV
    ```

## Checklist

- [ ] Re-run `GMP` test transaction (see final section in original deployment docs ([1](./2025-09-GMP-anchor.md), [2](./2025-09-ITS-anchor.md)).
- [ ] Run the [e2e repository](https://github.com/eigerco/axelar-solana-e2e) pipeline.
