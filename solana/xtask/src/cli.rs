use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use cmd::solana::SolanaContract;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::middleware::SignerMiddleware;
use ethers::signers::coins_bip39::English;
use ethers::signers::{LocalWallet, MnemonicBuilder, Signer};
use ethers::types::Address;
use eyre::Context;
use k256::SecretKey;
use url::Url;

use self::cmd::cosmwasm::cosmos_client::signer::SigningClient;
use self::cmd::cosmwasm::{AXELAR_ACCOUNT_PREFIX, AXELAR_DEVNET};
use self::cmd::testnet::SOLANA_CHAIN_NAME;

pub(crate) mod cmd;

/// Xtask is the Axelar Solana workspace CLI that helps
/// both actors, humans and CI to achieve mundane tasks
/// like building, deploying and initializing Solana
/// programs.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) enum Cli {
    /// Build, deploy, instantiate and interact with our Solana programs
    Solana {
        #[command(subcommand)]
        command: Solana,
    },
    /// Delpoy, instantiate and operate with evm chains and our demo contracts
    Evm {
        #[arg(short, long)]
        source_evm_chain: String,
        /// The private key of the account that will send the tx
        #[arg(short, long)]
        admin_private_key: String,
        /// The command to execute
        #[command(subcommand)]
        command: Evm,
    },
    /// Work with cosmwasm contracts and the axelar chain
    Cosmwasm {
        #[command(subcommand)]
        command: Cosmwasm,
    },
    Testnet {
        #[command(subcommand)]
        command: TestnetFlowDirection,
    },
    GenerateEvm,
}

#[derive(Subcommand)]
pub(crate) enum TestnetFlowDirection {
    EvmToEvm {
        #[arg(long)]
        source_evm_private_key_hex: String,
        #[arg(long)]
        axelar_private_key_hex: String,
        #[arg(long)]
        source_memo_contract: Option<Address>,
        #[arg(long)]
        destination_memo_contract: Option<Address>,
        #[arg(long)]
        destination_evm_private_key_hex: String,
        #[arg(long)]
        source_evm_chain: String,
        #[arg(long)]
        destination_evm_chain: String,
        #[arg(long)]
        memo_to_send: String,
    },
    EvmToSolana {
        // -- solana configs --
        /// The RPC URL of the target validator. If not provided, it will
        /// fallback into Solana CLI defaults.
        #[arg(short, long)]
        solana_rpc_url: Option<Url>,
        #[arg(short, long)]
        keypair_path: Option<PathBuf>,
        // -- axelar configs --
        #[arg(long)]
        axelar_private_key_hex: String,
        // -- evm configs --
        #[arg(long)]
        source_evm_private_key_hex: String,
        #[arg(long)]
        source_memo_contract: Address,
        #[arg(long)]
        source_evm_chain: String,
        #[arg(long)]
        memo_to_send: String,
    },
    SolanaToEvm {
        // -- solana configs --
        /// The RPC URL of the target validator. If not provided, it will
        /// fallback into Solana CLI defaults.
        #[arg(short, long)]
        solana_rpc_url: Option<Url>,
        #[arg(short, long)]
        keypair_path: Option<PathBuf>,
        #[arg(long)]
        memo_to_send: String,
        // -- axelar configs --
        #[arg(long)]
        axelar_private_key_hex: String,
        // -- evm configs --
        #[arg(long)]
        destination_evm_private_key_hex: String,
        #[arg(long)]
        destination_memo_contract: Address,
        #[arg(long)]
        destination_evm_chain: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum Cosmwasm {
    /// Build all cosmwasm contracts so that they would be ready for deployment
    Build,
    /// Deploy
    Deploy {
        #[arg(short, long)]
        private_key_hex: String,
    },
    Init {
        #[arg(short, long)]
        code_id: u64,
        #[arg(short, long)]
        private_key_hex: String,
        #[command(subcommand)]
        command: CosmwasmInit,
    },
    /// Generate a new Axelar wallet, outputs the Axelar bech32 key and the hex
    /// private key
    GenerateWallet,
    /// Bond an ampd verifier in an interactive process that follows [the official guide](https://docs.axelar.dev/validator/amplifier/verifier-onboarding)
    /// The chain `devnet-amplifier` is assumed.
    AmpdSetup,
    /// Start ampd and tofnd at the same time
    AmpdAndTofndRun,
}

/// Initialize contracts by providing their specific init parameters.
#[derive(Subcommand)]
pub(crate) enum CosmwasmInit {
    /// Initialize an already deployed voting verifier contract.
    SolanaVotingVerifier,
    /// Initialize an already deployed gateway contract
    Gateway {
        #[arg(short, long)]
        voting_verifier_address: String,
    },
    // Initialize an already deployed multisig prover contract.
    SolanaMultisigProver {
        #[arg(long)]
        gateway_address: String,
        #[arg(long)]
        voting_verifier_address: String,
    },
}
/// The contracts are pre-built as ensured by the `evm-contracts-rs` crate in
/// our workspace. On EVM we don't differentiate deployment fron initialization
/// as we do on Solana.
#[derive(Subcommand)]
pub(crate) enum Evm {
    DeployAxelarMemo {},
    SendMemoToSolana {
        #[arg(short, long)]
        evm_memo_contract_address: ethers::types::Address,
        #[arg(short, long)]
        memo_to_send: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum Solana {
    /// Build's a contract that is listed in the programs
    /// workspace directory.
    Build {
        /// It accepts the name of the contract folder as argument.
        #[arg(value_enum)]
        contract: SolanaContract,
    },
    /// Deploys the given contract name
    Deploy {
        /// It accepts the name of the contract folder as argument.
        #[arg(value_enum)]
        contract: SolanaContract,
        /// They keypair used to deploy the contract and sign transactions.
        /// If not provided, it will fallback into Solana CLI defaults.
        #[arg(short, long)]
        keypair_path: Option<PathBuf>,
        /// The RPC URL of the target validator. If not provided, it will
        /// fallback into Solana CLI defaults.
        #[arg(short, long)]
        url: Option<Url>,
        /// The websocket URL of the target validator. Normally the same as the
        /// rpc url, but replacing scheme in favour of ws:// . If not
        /// provided, it will fallback into Solana CLI defaults.
        #[arg(short, long)]
        ws_url: Option<Url>,
        /// The file path to the solana program that's associated with the
        /// hardcoded program id
        #[arg(short, long)]
        program_id: PathBuf,
        // ---
        // TODO: expose "upgrate_authority"
    },
    Init {
        #[command(subcommand)]
        contract: SolanaInitSubcommand,
    },
}

/// Initialize contracts by providing their specific init parameters.
#[derive(Subcommand)]
pub(crate) enum SolanaInitSubcommand {
    /// Initialize an already deployed gateway contract.
    GmpGateway {
        /// A path that points to a toml file that contains the signers and
        /// their respective weights data. See `tests/auth_weighted.toml` file
        /// for an example.
        #[arg(short, long)]
        auth_weighted_file: PathBuf,
        /// The RPC URL of the target validator.
        /// If not provided, this will fallback in solana CLI current
        /// configuration.
        #[arg(short, long)]
        rpc_url: Option<Url>,
        /// The payer keypair file. This is a file containing the byte slice
        /// serialization of a `solana_sdk::signer::keypair::Keypair` .
        /// If not provided, this will fallback in solana CLI current
        /// configuration.
        #[arg(short, long)]
        payer_kp_path: Option<PathBuf>,
    },
}

impl Cli {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        match self {
            Cli::Solana { command } => handle_solana(command).await?,
            Cli::Evm {
                source_evm_chain,
                admin_private_key,
                command,
            } => handle_evm(source_evm_chain, admin_private_key, command).await?,
            Cli::Cosmwasm { command } => handle_cosmwasm(command).await?,
            Cli::Testnet { command } => handle_testnet(command).await?,
            Cli::GenerateEvm => handle_generating_evm_wallet()?,
        };
        Ok(())
    }
}

#[allow(clippy::too_many_lines)]
async fn handle_testnet(command: TestnetFlowDirection) -> eyre::Result<()> {
    match command {
        TestnetFlowDirection::EvmToEvm {
            source_evm_private_key_hex: source_evm_private_key,
            axelar_private_key_hex,
            destination_evm_private_key_hex: destination_evm_private_key,
            source_evm_chain,
            destination_evm_chain,
            memo_to_send,
            source_memo_contract,
            destination_memo_contract,
        } => {
            let source_chain = get_evm_chain(source_evm_chain.as_str())?;
            let destination_chain = get_evm_chain(destination_evm_chain.as_str())?;

            let source_evm_signer = create_evm_signer(&source_chain, source_evm_private_key).await;
            let destination_evm_signer =
                create_evm_signer(&source_chain, destination_evm_private_key).await;

            let cosmwasm_signer = create_axelar_cosmsos_signer(axelar_private_key_hex)?;
            let source_memo_contract =
                get_or_deploy_evm_contract(source_memo_contract, &source_evm_signer, &source_chain)
                    .await?;
            let destination_memo_contract = get_or_deploy_evm_contract(
                destination_memo_contract,
                &destination_evm_signer,
                &destination_chain,
            )
            .await?;

            cmd::testnet::evm_to_evm(
                &source_chain,
                &destination_chain,
                source_memo_contract,
                destination_memo_contract,
                source_evm_signer,
                destination_evm_signer,
                memo_to_send,
                cosmwasm_signer,
            )
            .await?;
        }
        TestnetFlowDirection::EvmToSolana {
            solana_rpc_url,
            keypair_path,
            axelar_private_key_hex,
            source_evm_private_key_hex,
            source_memo_contract,
            source_evm_chain,
            memo_to_send,
        } => {
            let source_chain = get_evm_chain(source_evm_chain.as_str())?;
            let source_evm_signer =
                create_evm_signer(&source_chain, source_evm_private_key_hex).await;
            let cosmwasm_signer = create_axelar_cosmsos_signer(axelar_private_key_hex)?;
            let source_memo_contract = get_or_deploy_evm_contract(
                Some(source_memo_contract),
                &source_evm_signer,
                &source_chain,
            )
            .await?;
            let solana_rpc_client = solana_client::rpc_client::RpcClient::new(
                cmd::solana::defaults::rpc_url_with_fallback_in_sol_cli_config(&solana_rpc_url)?
                    .to_string(),
            );
            let solana_keypair =
                cmd::solana::defaults::payer_kp_with_fallback_in_sol_cli_config(&keypair_path)?;
            cmd::testnet::evm_to_solana(
                &source_chain,
                source_evm_signer,
                cosmwasm_signer,
                source_memo_contract,
                solana_rpc_client,
                solana_keypair,
                memo_to_send,
            )
            .await?;
        }
        TestnetFlowDirection::SolanaToEvm {
            solana_rpc_url,
            keypair_path,
            memo_to_send,
            axelar_private_key_hex,
            destination_evm_private_key_hex,
            destination_memo_contract,
            destination_evm_chain,
        } => {
            let destination_chain = get_evm_chain(destination_evm_chain.as_str())?;
            let destination_evm_signer =
                create_evm_signer(&destination_chain, destination_evm_private_key_hex).await;
            let cosmwasm_signer = create_axelar_cosmsos_signer(axelar_private_key_hex)?;
            let destination_memo_contract = get_or_deploy_evm_contract(
                Some(destination_memo_contract),
                &destination_evm_signer,
                &destination_chain,
            )
            .await?;
            let solana_rpc_client = solana_client::rpc_client::RpcClient::new(
                cmd::solana::defaults::rpc_url_with_fallback_in_sol_cli_config(&solana_rpc_url)?
                    .to_string(),
            );
            let solana_keypair =
                cmd::solana::defaults::payer_kp_with_fallback_in_sol_cli_config(&keypair_path)?;
            cmd::testnet::solana_to_evm(
                &destination_chain,
                destination_evm_signer,
                cosmwasm_signer,
                destination_memo_contract,
                solana_rpc_client,
                solana_keypair,
                memo_to_send,
            )
            .await?;
        }
    }

    Ok(())
}

async fn get_or_deploy_evm_contract(
    memo_contract: Option<ethers::types::H160>,
    evm_signer: &evm_contracts_test_suite::EvmSigner,
    chain: &cmd::testnet::devnet_amplifier::EvmChain,
) -> Result<ethers::types::H160, eyre::Error> {
    let destination_memo_contract = if let Some(addr) = memo_contract {
        addr
    } else {
        tracing::info!(chain = ?chain.id, "memo contract not present, deploying");
        let res =
            cmd::evm::deploy_axelar_memo(evm_signer.clone(), chain.axelar_gateway.parse().unwrap())
                .await?;
        tracing::info!("sleeping for 10 seconds for the change to propagate");
        tokio::time::sleep(Duration::from_secs(10)).await;
        res
    };
    Ok(destination_memo_contract)
}

async fn handle_solana(command: Solana) -> eyre::Result<()> {
    match command {
        Solana::Build { contract } => {
            cmd::solana::build_contract(contract)?;
        }
        Solana::Deploy {
            contract,
            keypair_path,
            url,
            ws_url,
            program_id,
        } => {
            cmd::solana::deploy(contract, program_id.as_path(), &keypair_path, &url, &ws_url)?;
        }
        Solana::Init { contract } => match &contract {
            SolanaInitSubcommand::GmpGateway {
                auth_weighted_file,
                rpc_url,
                payer_kp_path,
            } => {
                cmd::solana::init_gmp_gateway(auth_weighted_file, rpc_url, payer_kp_path).await?;
            }
        },
    };
    Ok(())
}

async fn handle_evm(chain: String, admin_private_key: String, command: Evm) -> eyre::Result<()> {
    let chain = get_evm_chain(chain.as_str())?;
    let signer = create_evm_signer(&chain, admin_private_key).await;
    match command {
        Evm::DeployAxelarMemo {} => {
            get_or_deploy_evm_contract(None, &signer, &chain).await?;
        }
        Evm::SendMemoToSolana {
            evm_memo_contract_address,
            memo_to_send,
        } => {
            cmd::evm::send_memo_to_solana(
                signer,
                evm_memo_contract_address,
                memo_to_send.as_str(),
                SOLANA_CHAIN_NAME,
            )
            .await?;
        }
    };
    Ok(())
}
async fn handle_cosmwasm(command: Cosmwasm) -> eyre::Result<()> {
    match command {
        Cosmwasm::Build => {
            cmd::cosmwasm::build().await?;
        }
        Cosmwasm::Deploy { private_key_hex } => {
            let cosmwasm_signer = create_axelar_cosmsos_signer(private_key_hex)?;
            cmd::cosmwasm::deploy(cosmwasm_signer).await?;
        }
        Cosmwasm::GenerateWallet => cmd::cosmwasm::generate_wallet()?,
        Cosmwasm::Init {
            code_id,
            command,
            private_key_hex,
        } => {
            let client = create_axelar_cosmsos_signer(private_key_hex)?;
            match command {
                CosmwasmInit::SolanaVotingVerifier => {
                    cmd::cosmwasm::init_solana_voting_verifier(code_id, client).await?;
                }
                CosmwasmInit::Gateway {
                    voting_verifier_address,
                } => {
                    cmd::cosmwasm::init_gateway(code_id, client, voting_verifier_address).await?;
                }
                CosmwasmInit::SolanaMultisigProver {
                    gateway_address,
                    voting_verifier_address,
                } => {
                    cmd::cosmwasm::init_solana_multisig_prover(
                        code_id,
                        client,
                        gateway_address,
                        voting_verifier_address,
                    )
                    .await?;
                }
            }
        }
        Cosmwasm::AmpdSetup => cmd::cosmwasm::ampd::setup_ampd().await?,
        Cosmwasm::AmpdAndTofndRun => cmd::cosmwasm::ampd::start_with_tofnd().await?,
    };
    Ok(())
}

fn handle_generating_evm_wallet() -> eyre::Result<()> {
    let mnemonic = bip39::Mnemonic::generate(24)?;
    let words = mnemonic.word_iter().collect::<Vec<_>>().join(" ");
    let mnemonic = MnemonicBuilder::<English>::default()
        .phrase(ethers::types::PathOrString::String(words.clone()));
    println!("mnemonic: {words}");

    // Derive the first 3 private and public keys from the seed
    for i in 0_u32..3_u32 {
        let pk = mnemonic.clone().index(i).unwrap().build().unwrap();
        let addr = pk.address();
        let private_key = pk.signer().to_bytes();

        println!("Key {i}: ");
        println!("  Private Key: 0x{}", hex::encode(private_key));
        println!("  Address: 0x{}", hex::encode(addr.to_fixed_bytes()));
    }

    let chains = cmd::testnet::devnet_amplifier::get_chains();
    let allowed_chains = chains.keys().collect::<Vec<_>>();
    println!("supported chains for axelar testnet operations: {allowed_chains:?}");
    Ok(())
}
async fn init_evm_signer(
    node_rpc: &Url,
    wallet: LocalWallet,
) -> Arc<
    SignerMiddleware<
        Arc<ethers::providers::Provider<ethers::providers::Http>>,
        ethers::signers::Wallet<SigningKey>,
    >,
> {
    let provider =
        ethers::providers::Provider::<ethers::providers::Http>::try_from(node_rpc.as_str())
            .expect("URL is always valid")
            .interval(std::time::Duration::from_millis(200));
    let provider = Arc::new(provider);
    let client = SignerMiddleware::new_with_provider_chain(provider, wallet)
        .await
        .unwrap();

    Arc::new(client)
}

async fn create_evm_signer(
    chain: &cmd::testnet::devnet_amplifier::EvmChain,
    private_key_hex: String,
) -> evm_contracts_test_suite::EvmSigner {
    let private_key =
        SecretKey::from_slice(hex::decode(private_key_hex).unwrap().as_ref()).unwrap();
    let wallet = LocalWallet::from_bytes(private_key.to_bytes().as_ref()).unwrap();
    let source_signer = init_evm_signer(&chain.rpc, wallet.clone()).await;

    evm_contracts_test_suite::EvmSigner {
        wallet: wallet.clone(),
        signer: source_signer,
    }
}

fn create_axelar_cosmsos_signer(
    axelar_private_key_hex: String,
) -> Result<SigningClient, eyre::Error> {
    let key_bytes = hex::decode(axelar_private_key_hex)?;
    let signing_key = cosmrs::crypto::secp256k1::SigningKey::from_slice(&key_bytes)
        .context("invalid secp256k1 private key")?;
    let cosmwasm_signer = SigningClient {
        network: AXELAR_DEVNET.clone(),
        account_prefix: AXELAR_ACCOUNT_PREFIX.to_owned(),
        signing_key,
    };
    Ok(cosmwasm_signer)
}

fn get_evm_chain(evm_chain: &str) -> eyre::Result<cmd::testnet::devnet_amplifier::EvmChain> {
    let chains = cmd::testnet::devnet_amplifier::get_chains();
    let chain = chains
        .get(evm_chain)
        .ok_or_else(|| {
            let allowed_chains = chains.keys().collect::<Vec<_>>();
            eyre::eyre!("allowed chain values are {allowed_chains:?}")
        })?
        .clone();

    tracing::info!(?chain, "resolved evm chain");
    Ok(chain.clone())
}
