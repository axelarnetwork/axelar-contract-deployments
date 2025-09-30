# Solana ITS (anchor)

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

| **Axelar Env**       | **Solana Env** | **PDA**                                       |
| -------------------- | -------------- | --------------------------------------------- |
| **Devnet Amplifier** | **Devnet**     | `` |
| **Stagenet**         | **Testnet**    | `` |
| **Testnet**          | **Testnet**    | `` |
| **Mainnet**          | **Mainnet**    | `` |

## Background

This is the anchor Solana ITS release.

## Deployment

### Deployment Prerequisites

1. Ensure you have access to the keypairs for the following accounts, unless you plan to generate new keypairs (below):

    | Axelar Env           | Authority                          | Operator                           |
    | -------------------- | ---------------------------------- | ---------------------------------- |
    | **Devnet-amplifier** | `<generate key with 'upa' prefix>` | `<generate key with 'gop' prefix>` |
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
        mv <generated upa keypair>.json /path/to/my/keys/folder/upgrade-authority-$ENV.json

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

1. Ensure the [`spl-token`](https://crates.io/crates/spl-token-cli) CLI is installed.

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

1. Compile the Solana programs:

    ```sh
    # Go to the solana directory within the cloned repo
    cd axelar-amplifier-solana

    # Compile the ITS
    solana-verify build --base-image $BASE_IMAGE --library-name axelar_solana_its

    # Go back
    cd ..
    ```

1. Declare additional environment variables in your `.env`:

    ```sh
    ITS_PROGRAM_KEYPAIR_PATH="<path/to/its_program_keypair.json>"
    ITS_PROGRAM_PATH="axelar-amplifier-solana/target/deploy/axelar_solana_its.so"
    ```

    ```bash
    source .env
    ```

### Deployment Steps

> [!NOTE]
> If `--upgrade-authority` is omitted, the current Solana CLI keypair is set as upgrade-authority.
> After each deployment, populate the above tables with the respective PDAs.

1. Be sure you are in `axelar-contract-deployments/solana/axelar-amplifier-solana` directory.

1. Deploy and verify axelar_solana_its program (only run the `solana-verify` command for mainnet):

    ```sh
    anchor deploy -p axelar_solana_gateway --provider.cluster $CLUSTER --program-keypair ITS_PROGRAM_KEYPAIR_PATH -v -- --upgrade-authority $UPGRADE_AUTHORITY_KEYPAIR_PATH

    anchor verify -p axelar_solana_gateway --provider.cluster $CLUSTER $(solana address -k $ITS_PROGRAM_KEYPAIR_PATH) -- --no-default-features --features $ENV
    ```

### Initialization Steps

The initialization steps can only be performed by the upgrade authority.

1. Initialize ITS:

    ```sh
    solana/cli send its init --operator <ITS_OPERATOR_BASE58_PUBKEY>
    ```

1. Register Solana ITS on ITS Hub:

    ITS hub contract configuration in `axelar-chains-config/info/$ENV.json` must include the following attributes per chain:

    - For Devnet-amplifier / Stagenet / Testnet

        ```json
        "axelar": {
            "contracts": {
                "InterchainTokenService": {
                    "$CHAIN": {
                        "maxUintBits": 64,
                        "maxDecimalsWhenTruncating": 255
                    }
                }
            }
        }
        ```

    - For Mainnet, add a community post for the proposal.

        ```sh
        node cosmwasm/submit-proposal.js \
            its-hub-register-chains $CHAIN \
            -t "Register $CHAIN on ITS Hub" \
            -d "Register $CHAIN on ITS Hub"
        ```

1. Set up trusted chains on Solana:

    ```sh
    # Add all trusted chains to Solana ITS
    solana/cli send its set-trusted-chain all
    ```

    Alternatively, add specific chains:

    ```sh
    # Add a specific trusted chain
    solana/cli send its set-trusted-chain $CHAIN
    ```

1. Set Solana as trusted chain on EVM ITS and other chain ITS implementations:

    ```sh
    # Set PRIVATE_KEY in .env
    PRIVATE_KEY=<EVM_DEPLOYER_KEY>

    node evm/its.js set-trusted-chains $CHAIN hub -n all
    ```

## Checklist

The following checks should be performed after the rollout.

### Solana to EVM

> [!NOTE]
> Make sure you have an existing token with an associated Metaplex metadata account. You can create one using the [metaplex-foundation CLI](https://github.com/metaplex-foundation/cli).

1. Deploy Native Interchain Token:

    ```sh
    solana/cli send its deploy-interchain-token \
    --salt <SALT_STRING> \
    --name <TOKEN_NAME> \
    --symbol <TOKEN_SYMBOL> \
    --decimals <DECIMALS> \
    --initial-supply <INITIAL_SUPPLY>

    solana/cli send its deploy-remote-interchain-token \
    --salt <SALT_STRING> \
    --destination-chain <DESTINATION_CHAIN_NAME> \
    --gas-value <GAS_VALUE>
    ```

1. Interchain Token Transfer for Native Interchain Token:

    ```sh
    solana/cli send its interchain-transfer \
    --source-account <SOURCE_ACCOUNT_ADDRESS> \
    --token-id <TOKEN_ID_HEX> \
    --destination-chain <DESTINATION_CHAIN_NAME> \
    --destination-address <DESTINATION_ADDRESS> \
    --amount <AMOUNT> \
    --gas-value <GAS_VALUE>
    ```

1. Deploy Remote Canonical Token:

    ```sh
    solana/cli send its register-canonical-interchain-token \
    --mint <MINT_ADDRESS>

    solana/cli send its deploy-remote-canonical-interchain-token \
    --mint <MINT_ADDRESS> \
    --destination-chain <DESTINATION_CHAIN_NAME> \
    --gas-value <GAS_VALUE>
    ```

1. Interchain Token Transfer for Canonical Token:

    ```sh
    solana/cli send its interchain-transfer \
    --source-account <SOURCE_ACCOUNT_ADDRESS> \
    --token-id <TOKEN_ID_HEX> \
    --destination-chain <DESTINATION_CHAIN_NAME> \
    --destination-address <DESTINATION_ADDRESS> \
    --amount <AMOUNT> \
    --gas-value <GAS_VALUE>
    ```

### EVM to Solana

> [!TIP]
> You can get the mint address of an interchain token deployed on Solana using the `token_id` via:
>
> ```sh
> solana/cli query its token-manager <TOKEN_ID>
> ```

> [!IMPORTANT]
> When transferring tokens to Solana, the destination address should be a token account and not a wallet address. You can create a token account associated with a mint via:
>
> ```sh
> spl-token create-account --fee-payer <PAYER_KEYPAIR> --owner <WALLET_ADDRESS> <MINT_ADDRESS>
> ```
>
> When `--fee-payer` and/or `--owner` are omitted, the default Solana CLI keypair is used.

1. Deploy Native Interchain Token:

    ```sh
    node evm/interchainTokenFactory.js --action deployInterchainToken -n <SOURCE_CHAIN> --destinationChain $CHAIN --salt <SALT> --name <TOKEN_NAME> --symbol <TOKEN_SYMBOL> --decimals <DECIMALS>

    node evm/interchainTokenFactory.js --action deployRemoteInterchainToken -n <SOURCE_CHAIN> --destinationChain $CHAIN --salt <SALT> --gasValue <GAS_VALUE>
    ```

1. Interchain Token Transfer for Native Interchain Token:

    ```sh
    node evm/its.js -n <SOURCE_CHAIN> interchain-transfer $CHAIN <TOKEN_ID> <ENCODED_RECIPIENT> <AMOUNT>
    ```

1. Deploy Remote Canonical Token:

    ```sh
    node evm/interchainTokenFactory.js --action registerCanonicalInterchainToken -n <SOURCE_CHAIN> --destinationChain $CHAIN --tokenAddress <TOKEN_ADDRESS>

    node evm/interchainTokenFactory.js --action deployRemoteCanonicalInterchainToken -n <SOURCE_CHAIN> --destinationChain $CHAIN --tokenAddress <TOKEN_ADDRESS> --gasValue <GAS_VALUE>
    ```

1. Interchain Token Transfer for Canonical Token:

    ```sh
    node evm/its.js interchain-transfer --gasValue <GAS_VALUE> $CHAIN <TOKEN_ID> <ENCODED_RECIPIENT> <AMOUNT>
    ```
