# Solana GMP (vanilla)

## Network Status

|                | **Owner**    |
| -------------- | ------------ |
| **Created By** | @nbayindirli |
| **Deployment** | @nbayindirli |

| **Axelar Env**       | **Deployment Status** | **Date**   |
| -------------------- | --------------------- | ---------- |
| **Devnet Amplifier** | Completed             | 2025-09-18 |
| **Stagenet**         | Pending               | TBD        |
| **Testnet**          | Pending               | TBD        |
| **Mainnet**          | Pending               | TBD        |

## PDAs

- Add the below PDAs as they are generated during the release process.

### Devnet Amplifier

| **Axelar Env**       | **Program** | **Solana Env** | **PDA**                                       |
| -------------------- | ----------- | -------------- | --------------------------------------------- |
| **Devnet Amplifier** | Gateway     | **Devnet**     | `gtwRtcJ6CT8a7bsymztPEDzaMM5d3h6PN6vXeHtkESq` |
| **Devnet Amplifier** | Gas Service | **Devnet**     | `gasfiUSLzSZEKQiH4Nsm5Yk2rjzWWEtsk3Ji3qs1Ryw` |
| **Devnet Amplifier** | Governance  | **Devnet**     | `govWBE74GceA32mcz9CXuFV1zLvtSgkqXh4czqondBi` |
| **Devnet Amplifier** | Multicall   | **Devnet**     | `mcxMSFXDGWrRDXiWWcuFnn2HpmTnmbPw9MYh3MmWSNw` |

### Stagenet

| **Axelar Env** | **Program** | **Solana Env** | **PDA**                                       |
| -------------- | ----------- | -------------- | --------------------------------------------- |
| **Stagenet**   | Gateway     | **Testnet**    | `` |
| **Stagenet**   | Gas Service | **Testnet**    | `` |
| **Stagenet**   | Governance  | **Testnet**    | `` |
| **Stagenet**   | Multicall   | **Testnet**    | `` |

### Testnet

| **Axelar Env** | **Program** | **Solana Env** | **PDA**                                       |
| -------------- | ----------- | -------------- | --------------------------------------------- |
| **Testnet**    | Gateway     | **Testnet**    | `` |
| **Testnet**    | Gas Service | **Testnet**    | `` |
| **Testnet**    | Governance  | **Testnet**    | `` |
| **Testnet**    | Multicall   | **Testnet**    | `` |

### Mainnet

| **Axelar Env** | **Program** | **Solana Env** | **PDA**                                       |
| -------------- | ----------- | -------------- | --------------------------------------------- |
| **Mainnet**    | Gateway     | **Mainnet**    | `` |
| **Mainnet**    | Gas Service | **Mainnet**    | `` |
| **Mainnet**    | Governance  | **Mainnet**    | `` |
| **Mainnet**    | Multicall   | **Mainnet**    | `` |

## Background

This is the vanilla Solana GMP release.

## Deployment

### Deployment Prerequisites

1. Ensure you have access to the keypairs for the following accounts, unless you plan to generate new keypairs (below):

    | Axelar Env           | Authority                                     | Operator                                      |
    | -------------------- | --------------------------------------------- | --------------------------------------------- |
    | **Devnet-amplifier** | `upa8CAJAvxU32TZfVT6mcHQawRLzx3N4c65GQjL8Vfx` | `gopEFNgirbVNK29RA5DK8mZTDhN2whzcbhCWXkVEc18` |
    | **Stagenet**         | `<generate key with 'upa' prefix>` | `<generate key with 'gop' prefix>` |
    | **Testnet**          | `<generate key with 'upa' prefix>` | `<generate key with 'gop' prefix>` |
    | **Mainnet**          | `<generate key with 'upa' prefix>` | `<generate key with 'gop' prefix>` |

1. Create an `.env` config with the following:

    ```sh
    # Used by 'solana-verify' tool get verifiable builds
    BASE_IMAGE="solanafoundation/solana-verifiable-build@sha256:979b09eef544de4502a92e28a724a8498a08e2fe506e8905b642e613760403d3"
    COMMIT_HASH="<latest axelar-amplifier-solana commit hash>"
    ENV=<devnet-custom|devnet-amplifier|stagenet|testnet|mainnet>
    CLUSTER=<devnet|testnet|mainnet-beta>
    CHAIN=<solana-custom|solana>
    ```

1. Ensure you have Rust installed. If you don't:

    ```sh
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

1. Install Solana CLI:

    ```sh
    sh -c "$(curl -sSfL https://release.anza.xyz/v2.2.14/install)"
    ```

1. Install `solana-verify`, for verifiable builds:

    ```sh
    cargo install solana-verify
    ```

1. Set default Solana cluster:

    ```sh
    # Set default cluster
    solana config set --url $CLUSTER
    ```

1. Generate or set existing keypair:

    - To generate and set a new keypair:

        ```sh
        # Get keypair starting with 'upa'
        solana-keygen grind --starts-with upa:1

        # Move new keypair out of current dir
        mv [generated upa keypair].json /path/to/my/keys/folder/upgrade-authority-$ENV.json

        # Set to new keypair
        solana config set --keypair /path/to/my/keys/folder/upgrade-authority-$ENV.json
        ```

    - To set an existing keypair:

        ```sh
        solana config set --keypair /path/to/my/keys/folder/upgrade-authority-$ENV.json
        ```

1. Fund keypair (unless on mainnet)

    ```sh
    solana airdrop 5
    ```

1. Get or verify account address for your keypair:

    ```sh
    solana address
    ```

1. Verify account is funded:

    ```sh
    solana balance
    ```

    Note: If funding is not enough, reach out to a team member for SOL.

### Deployment Setup

> [!IMPORTANT]
> For the initial deployment of Solana programs to any of the clusters (devnet, testnet, and mainnet-beta), the program keypairs are required. The pubkey is the program ID and is hardcoded in the program using the `declare_id` macro. In case a new set of keypairs is required, a new release of the crates needs to happen afterwards (due to the id being hardcoded). Updating the ids can be done within the `axelar-amplifier-solana` directory by invoking:
>
> ```sh
> cargo xtask update-ids
> ```
>
> The keypair files should be stored securely as they're needed for the initial deployment on other clusters as well.

> [!NOTE]
> Initial deployment of Solana programs doesn't support offline signing, the process needs to be done online. When deploying, an `upgrade-authority` can be set, which will later be able to perform program upgrades — upgrades support offline signing.

1. Clone the [`axelar-amplifier-solana`](https://github.com/axelarnetwork/axelar-amplifier-solana) repo.

1. Check out the `upstream` branch.

    - Note: This will change for real V1 release.

1. Compile the Solana programs:

    ```sh
    # Go to the solana directory within the cloned repo
    cd axelar-amplifier-solana

    # Compile the Solana programs
    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_gas_service
    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_gateway
    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_governance
    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_multicall
    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_memo_program

    # Go back
    cd ..
    ```

1. Declare additional environment variables in your `.env`:

    ```sh
    GATEWAY_PROGRAM_KEYPAIR_PATH="<path/to/gateway_program_keypair.json>"
    GATEWAY_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_gateway.so"

    GAS_SERVICE_PROGRAM_KEYPAIR_PATH="<path/to/gas_service_program_keypair.json>"
    GAS_SERVICE_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_gas_service.so"

    GOVERNANCE_PROGRAM_KEYPAIR_PATH="<path/to/governance_program_keypair.json>"
    GOVERNANCE_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_governance.so"

    MULTICALL_PROGRAM_KEYPAIR_PATH="<path/to/multicall_program_keypair.json>"
    MULTICALL_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_multicall.so"

    MEMO_PROGRAM_KEYPAIR_PATH="<path/to/memo_program_keypair.json>"
    MEMO_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_memo_program.so"

    UPGRADE_AUTHORITY_KEYPAIR_PATH="<path/to/upgrade_authority_keypair.json>"
    ```

    ```bash
    source .env
    ```

1. Add Solana chain config to the `axelar-chains-config/info/$ENV.json` file under the `chains` key:

    ```json
        "$CHAIN": {
            "name": "Solana $ENV",
            "axelarId": "$CHAIN",
            "rpc": "https://api.$CLUSTER.solana.com",
            "chainType": "svm",
            "decimals": 9,
            "finality": "31",
            "approxFinalityWaitTime": 1,
            "tokenSymbol": "SOL",
            "explorer": {
                "name": "Solana $CLUSTER Explorer",
                "url": "https://explorer.solana.com/?cluster=$CLUSTER"
            },
            "contracts": {}
        }
    ```

### Deployment Steps

> [!NOTE]
> If `--upgrade-authority` is omitted, the current Solana CLI keypair is set as upgrade-authority.
> After each deployment, populate the above tables with the respective PDAs.
>
> Only run the `solana-verify` commands for mainnet

1. Be sure you are back in `axelar-contract-deployments/solana` directory.

1. Deploy and verify axelar_solana_gateway program:

    ```sh
    solana program deploy --program-id $GATEWAY_PROGRAM_KEYPAIR_PATH --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $GATEWAY_PROGRAM_PATH

    solana-verify verify-from-repo https://github.com/axelarnetwork/axelar-amplifier-solana --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $GATEWAY_PROGRAM_KEYPAIR_PATH) --library-name axelar_solana_gateway -- --no-default-features --features $ENV
    ```

1. Deploy and verify axelar_solana_gas_service program:

    ```sh
    solana program deploy --program-id $GAS_SERVICE_PROGRAM_KEYPAIR_PATH --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $GAS_SERVICE_PROGRAM_PATH

    solana-verify verify-from-repo https://github.com/axelarnetwork/axelar-amplifier-solana --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $GAS_SERVICE_PROGRAM_KEYPAIR_PATH) --library-name axelar_solana_gas_service -- --no-default-features --features $ENV
    ```

1. Deploy and verify axelar_solana_governance program:

    ```sh
    solana program deploy --program-id $GOVERNANCE_PROGRAM_KEYPAIR_PATH --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $GOVERNANCE_PROGRAM_PATH

    solana-verify verify-from-repo https://github.com/axelarnetwork/axelar-amplifier-solana --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $GOVERNANCE_PROGRAM_KEYPAIR_PATH) --library-name axelar_solana_governance -- --no-default-features --features $ENV
    ```

1. Deploy and verify axelar_solana_multicall program:

    ```sh
    solana program deploy --program-id $MULTICALL_PROGRAM_KEYPAIR_PATH --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $MULTICALL_PROGRAM_PATH

    solana-verify verify-from-repo https://github.com/axelarnetwork/axelar-amplifier-solana --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $MULTICALL_PROGRAM_KEYPAIR_PATH) --library-name axelar_solana_multicall -- --no-default-features --features $ENV
    ```

1. Deploy and verify axelar_solana_memo_program program:

    ```sh
    solana program deploy --program-id $MEMO_PROGRAM_KEYPAIR_PATH --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $MEMO_PROGRAM_PATH

    solana-verify verify-from-repo https://github.com/axelarnetwork/axelar-amplifier-solana --remote --base-image $BASE_IMAGE --commit-hash $COMMIT_HASH --program-id $(solana address -k $MEMO_PROGRAM_KEYPAIR_PATH) --library-name axelar_solana_memo_program -- --no-default-features --features $ENV
    ```

1. After deploying Solana contracts, deploy the [Solana GMP Amplifier](../cosmwasm/2025-09-Solana-GMP-v1.0.0.md).

### Initialization Steps

The initialization steps can only be performed by the upgrade authority.

1. Based on the PDA tables, set the following variables:

    ```sh
    UPGRADE_AUTHORITY_PDA="[upgrade-authority-pda]"
    OPERATOR_PDA="[operator-pda]"
    GATEWAY_PDA="[gateway-pda]"
    GAS_SERVICE_PDA="[gas-service-pda]"
    GOVERNANCE_PDA="[governance-pda]"
    MEMO_PDA="[memo-pda]"
    ```

1. Initialize Gateway:

    | Axelar Env           | `minimumRotationDelay` | `previousSignersRetention` | `minimumProposalEtaDelaySeconds` |
    | -------------------- | ---------------------- | -------------------------- | -------------------------------- |
    | **Devnet-amplifier** | `0`                    | `15`                       | `3600`                           |
    | **Stagenet**         | `300`                  | `15`                       | `3600`                           |
    | **Testnet**          | `3600`                 | `15`                       | `3600`                           |
    | **Mainnet**          | `86400`                | `15`                       | `3600`                           |

    ```sh
    solana/cli --chain-id $CHAIN send gateway init \
        --previous-signers-retention [previousSignersRetention] \
        --minimum-rotation-delay [minimumRotationDelay] \
        --operator $OPERATOR_PDA
    ```

    This will query the `SolanaMultisigProver` for the `VerifierSet`. Thus, the `SolanaMultisigProver` must be deployed before this step and its information available within the appropriate object in the `axelar-chains-config/info/<devnet-amplifier|testnet|mainnet>.json` file.

1. Initialize Gas Service:

    ```sh
    solana/cli --chain-id $CHAIN send --signer-keys $OPERATOR_KEYPAIR_PATH gas-service init \
        --operator $OPERATOR_PDA \
        --salt "[version] $ENV"
    ```

    **Note**: The `--signer-keys` parameter must be set to the keypair file that corresponds to the operator address specified in `--operator`. The gas service init instruction requires the operator to sign the transaction to prove they are the legitimate operator for the gas service.

1. Initialize Governance:

    ```sh
    solana/cli --chain-id $CHAIN send governance init \
        --governance-chain 'Axelarnet' \
        --governance-address [axelarnet governance address] \
        --minimum-proposal-eta-delay [minimumProposalEtaDelaySeconds] \
        --operator $OPERATOR_PDA
    ```

## Checklist

The following check should be performed after the rollout. It will send a GMP message from Solana to itself through the Axelar network.

### Checklist Prerequisites

1. Build and deploy the `Memo` program (ensure variables have the proper values):

    ```sh
    cd axelar-amplifier-solana/

    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_memo_program -- --no-default-features --features $ENV

    cd ..

    MEMO_PROGRAM_KEYPAIR_PATH="<path/to/memo_program_keypair.json>"
    MEMO_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_memo_program.so"

    solana program deploy --program-id $MEMO_PROGRAM_KEYPAIR_PATH --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH $MEMO_PROGRAM_PATH
    ```

1. Initialize the `Memo` program:

    ```sh
    solana/cli --chain-id $CHAIN send memo init
    ```

    You should see the `Memo` program id and it's `Counter Account` address in the output, as in the example below. You're going to need them in the next steps.

    ```sh
    ------------------------------------------
    ✅ Memo program (memPJFxP6H6bjEKpUSJ4KC7C4dKAfNE3xWrTpJBKDwN) initialization details:
    Counter Account: 3kKbQ5zXpzeigQLcw82durTRdhnQU7AjfvFhpjbbC8W6
    ------------------------------------------
    ```

### Verify GMP calls

1. Build a `Memo` program payload:

    ```sh
    GMP_PAYLOAD=$(solana/cli misc build-axelar-message \
        --accounts "[counter program ID]:false:true" \
        --payload "48656C6C6F21") # "Hello!" in hex
    ```

1. Perform the `call-contract` to the `Memo` program using the payload:

    ```sh
    solana/cli send gateway call-contract \
        --destination-chain $CHAIN \
        --destination-address $MEMO_PDA \
        --payload $GMP_PAYLOAD
    ```

1. If everything went well, you should see the broadcast information similar to the one below:

    ```sh
    Simulating transaction before sending...
    Simulation used 5012 compute units
    Transaction 1: 3xESUsYRvwDFM19yWSMPnmCWpbYCZHoqsHAnweBNosbegPzAQeTT9PsFfyUCKXhU8sprMW1V7dJwwoNDjpmk1yFD
    ------------------------------------------
    ✅ Solana Transaction successfully broadcast and confirmed!
    Transaction Signature (ID): 3xESUsYRvwDFM19yWSMPnmCWpbYCZHoqsHAnweBNosbegPzAQeTT9PsFfyUCKXhU8sprMW1V7dJwwoNDjpmk1yFD
    RPC Endpoint: https://api.devnet.solana.com
    Explorer Link: https://explorer.solana.com/tx/3xESUsYRvwDFM19yWSMPnmCWpbYCZHoqsHAnweBNosbegPzAQeTT9PsFfyUCKXhU8sprMW1V7dJwwoNDjpmk1yFD?cluster=devnet
    ------------------------------------------
    ```

1. Confirm message approval and execution on Axelarscan.
