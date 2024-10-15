use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use axelar_message_primitives::EncodingScheme;
use clap::{Parser, Subcommand};
use cmd::solana::SolanaContract;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::middleware::SignerMiddleware;
use ethers::signers::coins_bip39::English;
use ethers::signers::{LocalWallet, MnemonicBuilder, Signer};
use eyre::{Context, OptionExt};
use gmp_gateway::axelar_auth_weighted::RotationDelaySecs;
use k256::SecretKey;
use url::Url;

use self::cmd::axelar_deployments::{AxelarDeploymentRoot, EvmChain};
use self::cmd::cosmwasm::cosmos_client::signer::SigningClient;
use self::cmd::deployments::{
    AxelarConfiguration, CustomEvmChainDeployments, SolanaDeploymentRoot,
};
use crate::cli::cmd::path::xtask_crate_root_dir;

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
    /// Deploy, instantiate and operate with evm chains and our demo contracts
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
        destination_evm_private_key_hex: String,
        #[arg(long)]
        source_evm_chain: String,
        #[arg(long)]
        destination_evm_chain: String,
        #[arg(long)]
        memo_to_send: String,
    },
    EvmToSolana {
        #[arg(long)]
        memo_to_send: String,
        // -- axelar configs --
        #[arg(long)]
        axelar_private_key_hex: String,
        // -- evm configs --
        #[arg(long)]
        source_evm_private_key_hex: String,
        #[arg(long)]
        source_evm_chain: String,
    },
    SolanaToEvm {
        #[arg(long)]
        memo_to_send: String,
        // -- axelar configs --
        #[arg(long)]
        axelar_private_key_hex: String,
        // -- evm configs --
        #[arg(long)]
        destination_evm_private_key_hex: String,
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
        axelar_private_key_hex: String,
    },
    Init {
        #[arg(short, long)]
        axelar_private_key_hex: String,
        #[command(subcommand)]
        command: CosmwasmInit,
    },
    RedeployAndInitAll {
        #[arg(short, long)]
        axelar_private_key_hex: String,
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
    Gateway {},
    // Initialize an already deployed multisig prover contract.
    SolanaMultisigProver {},
    // Steup Multisig porver initial signers (post Axelar-governance approval)
    SolanaMultisigProverInitialSigners {},
}
/// The contracts are pre-built as ensured by the `evm-contracts-rs` crate in
/// our workspace. On EVM we don't differentiate deployment from initialization
/// as we do on Solana.
#[derive(Subcommand)]
pub(crate) enum Evm {
    DeployAxelarMemo {},
    SendMemoToSolana {
        #[arg(short, long)]
        memo_to_send: String,
    },
}

#[derive(Subcommand)]
pub(crate) enum Solana {
    /// Build's a contract that is listed in the programs
    /// workspace directory.
    Build,
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
        program_id_keypair_path: PathBuf,
        // ---
        // TODO: expose "upgrade_authority"
    },
    /// Iteratively send messages to the Solana Gateway, permuting different
    /// argument sizes and report the ones that succeed until the message
    /// limit is reached. The CSV report is written in the `output_dir`
    /// directory.
    MessageLimitsReport {
        /// Where to output the report
        output_dir: PathBuf,

        /// Enable ABI encoding scheme. When omitted, borsh
        /// encoding is used.
        #[arg(short, long)]
        abi_encoding: bool,
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
        #[arg(short, long)]
        axelar_private_key_hex: String,
        #[arg(short, long)]
        previous_signers_retention: u128,
        #[arg(short, long)]
        minimum_rotation_delay: RotationDelaySecs,
    },
    AxelarSolanaMemoProgram {},
}

impl Cli {
    pub(crate) async fn run(self) -> eyre::Result<()> {
        let solana_chain_name_on_axelar_chain = "solana-devnet".to_string();
        let axelar_chain_name = "devnet-amplifier";

        let axelar_deployment_root = get_axelar_configuration(axelar_chain_name)?;
        let mut solana_deployment_root = SolanaDeploymentRoot::new(
            solana_chain_name_on_axelar_chain,
            &axelar_deployment_root.axelar,
            cmd::solana::defaults::rpc_url()?.to_string(),
        )?;

        let res = match self {
            Cli::Solana { command } => handle_solana(command, &mut solana_deployment_root).await,
            Cli::Evm {
                source_evm_chain,
                admin_private_key,
                command,
            } => {
                handle_evm(
                    source_evm_chain,
                    admin_private_key,
                    command,
                    &axelar_deployment_root,
                    &mut solana_deployment_root,
                )
                .await
            }
            Cli::Cosmwasm { command } => {
                handle_cosmwasm(command, &mut solana_deployment_root).await
            }
            Cli::Testnet { command } => {
                handle_testnet(
                    command,
                    &axelar_deployment_root,
                    &mut solana_deployment_root,
                )
                .await
            }
            Cli::GenerateEvm => handle_generating_evm_wallet(&axelar_deployment_root),
        };
        solana_deployment_root.save()?;
        res?;
        Ok(())
    }
}

#[allow(clippy::too_many_lines)]
async fn handle_testnet(
    command: TestnetFlowDirection,
    axelar_deployment_root: &AxelarDeploymentRoot,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<()> {
    match command {
        TestnetFlowDirection::EvmToEvm {
            source_evm_private_key_hex: source_evm_private_key,
            axelar_private_key_hex,
            destination_evm_private_key_hex: destination_evm_private_key,
            source_evm_chain,
            destination_evm_chain,
            memo_to_send,
        } => {
            let source_chain = axelar_deployment_root.get_evm_chain(source_evm_chain.as_str())?;
            let destination_chain =
                axelar_deployment_root.get_evm_chain(destination_evm_chain.as_str())?;

            let source_evm_signer = create_evm_signer(&source_chain, source_evm_private_key).await;
            let destination_evm_signer =
                create_evm_signer(&source_chain, destination_evm_private_key).await;

            let cosmwasm_signer = create_axelar_cosmsos_signer(
                axelar_private_key_hex,
                &solana_deployment_root.axelar_configuration,
            )?;

            let source_evm_deployment_tracker = solana_deployment_root
                .evm_deployments
                .get_or_insert_mut(&source_chain);
            maybe_deploy_evm_memo_contract(
                &source_evm_signer,
                &source_chain,
                source_evm_deployment_tracker,
            )
            .await?;
            let destination_evm_deployment_tracker = solana_deployment_root
                .evm_deployments
                .get_or_insert_mut(&destination_chain);
            maybe_deploy_evm_memo_contract(
                &destination_evm_signer,
                &destination_chain,
                destination_evm_deployment_tracker,
            )
            .await?;

            cmd::testnet::evm_to_evm(
                &source_chain,
                &destination_chain,
                source_evm_signer,
                destination_evm_signer,
                memo_to_send,
                cosmwasm_signer,
                axelar_deployment_root,
                solana_deployment_root,
            )
            .await?;
        }
        TestnetFlowDirection::EvmToSolana {
            axelar_private_key_hex,
            source_evm_private_key_hex,
            source_evm_chain,
            memo_to_send,
        } => {
            let source_chain = axelar_deployment_root.get_evm_chain(source_evm_chain.as_str())?;
            let source_evm_signer =
                create_evm_signer(&source_chain, source_evm_private_key_hex).await;
            let cosmwasm_signer = create_axelar_cosmsos_signer(
                axelar_private_key_hex,
                &solana_deployment_root.axelar_configuration,
            )?;
            let source_evm_deployment_tracker = solana_deployment_root
                .evm_deployments
                .get_or_insert_mut(&source_chain);
            let _source_memo_contract = maybe_deploy_evm_memo_contract(
                &source_evm_signer,
                &source_chain,
                source_evm_deployment_tracker,
            )
            .await?;
            let solana_rpc_client = solana_client::rpc_client::RpcClient::new(
                cmd::solana::defaults::rpc_url()?.to_string(),
            );
            let solana_keypair = cmd::solana::defaults::payer_kp()?;
            cmd::testnet::evm_to_solana(
                &source_chain,
                source_evm_signer,
                cosmwasm_signer,
                solana_rpc_client,
                solana_keypair,
                memo_to_send,
                axelar_deployment_root,
                solana_deployment_root,
            )
            .await?;
        }
        TestnetFlowDirection::SolanaToEvm {
            memo_to_send,
            axelar_private_key_hex,
            destination_evm_private_key_hex,
            destination_evm_chain,
        } => {
            let destination_chain =
                axelar_deployment_root.get_evm_chain(destination_evm_chain.as_str())?;
            let destination_evm_signer =
                create_evm_signer(&destination_chain, destination_evm_private_key_hex).await;
            let cosmwasm_signer = create_axelar_cosmsos_signer(
                axelar_private_key_hex,
                &solana_deployment_root.axelar_configuration,
            )?;
            let destination_evm_deployment_tracker = solana_deployment_root
                .evm_deployments
                .get_or_insert_mut(&destination_chain);
            let destination_memo_contract = maybe_deploy_evm_memo_contract(
                &destination_evm_signer,
                &destination_chain,
                destination_evm_deployment_tracker,
            )
            .await?;
            let solana_rpc_client = solana_client::rpc_client::RpcClient::new(
                cmd::solana::defaults::rpc_url()?.to_string(),
            );
            let solana_keypair = cmd::solana::defaults::payer_kp()?;
            cmd::testnet::solana_to_evm(
                &destination_chain,
                destination_evm_signer,
                cosmwasm_signer,
                destination_memo_contract,
                solana_rpc_client,
                solana_keypair,
                memo_to_send,
                axelar_deployment_root,
                solana_deployment_root,
            )
            .await?;
        }
    }

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn maybe_deploy_evm_memo_contract(
    evm_signer: &evm_contracts_test_suite::EvmSigner,
    chain: &EvmChain,
    our_evm_deployment: &mut CustomEvmChainDeployments,
) -> Result<ethers::types::H160, eyre::Error> {
    if let Some(addr) = our_evm_deployment.memo_program_address.as_ref() {
        tracing::info!(?addr, "memo addr");
        return Ok(ethers::types::H160::from_str(addr)?);
    }

    tracing::info!(chain = ?chain.id, "memo contract not present, deploying");
    let res = cmd::evm::deploy_axelar_memo(
        evm_signer.clone(),
        chain
            .contracts
            .axelar_gateway
            .as_ref()
            .ok_or_eyre("gateway not deployed on this chain")?
            .address
            .parse()
            .unwrap(),
        our_evm_deployment,
    )
    .await?;
    tracing::info!("sleeping for 10 seconds for the change to propagate");
    tokio::time::sleep(Duration::from_secs(10)).await;
    Ok(res)
}

async fn handle_solana(
    command: Solana,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<()> {
    match command {
        Solana::Build => {
            cmd::solana::build_contracts(None)?;
        }
        Solana::Deploy {
            contract,
            keypair_path,
            url,
            ws_url,
            program_id_keypair_path: program_id,
        } => {
            cmd::solana::deploy(
                contract,
                program_id.as_path(),
                keypair_path.as_ref(),
                url.as_ref(),
                ws_url.as_ref(),
            )?;
        }
        Solana::MessageLimitsReport {
            output_dir,
            abi_encoding,
        } => {
            let encoding = if abi_encoding {
                EncodingScheme::AbiEncoding
            } else {
                EncodingScheme::Borsh
            };

            cmd::solana::generate_message_limits_report(&output_dir, encoding).await?;
        }
        Solana::Init { contract } => match contract {
            SolanaInitSubcommand::GmpGateway {
                axelar_private_key_hex,
                previous_signers_retention,
                minimum_rotation_delay,
            } => {
                let cosmwasm_signer = create_axelar_cosmsos_signer(
                    axelar_private_key_hex,
                    &solana_deployment_root.axelar_configuration,
                )?;
                cmd::solana::init_gmp_gateway(
                    cosmwasm_signer,
                    previous_signers_retention,
                    minimum_rotation_delay,
                    solana_deployment_root,
                )
                .await?;
            }
            SolanaInitSubcommand::AxelarSolanaMemoProgram {} => {
                cmd::solana::init_memo_program(solana_deployment_root)?;
            }
        },
    };
    Ok(())
}

async fn handle_evm(
    chain: String,
    admin_private_key: String,
    command: Evm,
    axelar_deployment_root: &AxelarDeploymentRoot,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<()> {
    let chain = axelar_deployment_root.get_evm_chain(chain.as_str())?;
    let signer = create_evm_signer(&chain, admin_private_key).await;
    match command {
        Evm::DeployAxelarMemo {} => {
            let deployment_tracker = solana_deployment_root
                .evm_deployments
                .get_or_insert_mut(&chain);
            maybe_deploy_evm_memo_contract(&signer, &chain, deployment_tracker).await?;
        }
        Evm::SendMemoToSolana { memo_to_send } => {
            let deployment_tracker = solana_deployment_root
                .evm_deployments
                .get_or_insert_mut(&chain);
            cmd::evm::send_memo_to_solana(
                signer,
                memo_to_send.as_str(),
                &solana_deployment_root
                    .solana_configuration
                    .chain_name_on_axelar_chain,
                deployment_tracker,
            )
            .await?;
        }
    };
    Ok(())
}
async fn handle_cosmwasm(
    command: Cosmwasm,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<()> {
    match command {
        Cosmwasm::Build => {
            cmd::cosmwasm::build().await?;
        }
        Cosmwasm::Deploy {
            axelar_private_key_hex: private_key_hex,
        } => {
            let cosmwasm_signer = create_axelar_cosmsos_signer(
                private_key_hex,
                &solana_deployment_root.axelar_configuration,
            )?;
            cmd::cosmwasm::deploy(
                &cosmwasm_signer,
                &mut solana_deployment_root.axelar_configuration,
            )
            .await?;
        }
        Cosmwasm::GenerateWallet => cmd::cosmwasm::generate_wallet()?,
        Cosmwasm::Init {
            command,
            axelar_private_key_hex: private_key_hex,
        } => {
            let client = create_axelar_cosmsos_signer(
                private_key_hex,
                &solana_deployment_root.axelar_configuration,
            )?;
            match command {
                CosmwasmInit::SolanaVotingVerifier => {
                    cmd::cosmwasm::init_solana_voting_verifier(&client, solana_deployment_root)
                        .await?;
                }
                CosmwasmInit::Gateway {} => {
                    cmd::cosmwasm::init_gateway(&client, solana_deployment_root).await?;
                }
                CosmwasmInit::SolanaMultisigProver {} => {
                    cmd::cosmwasm::init_solana_multisig_prover(&client, solana_deployment_root)
                        .await?;
                }
                CosmwasmInit::SolanaMultisigProverInitialSigners {} => {
                    cmd::cosmwasm::update_verifier_set_multisig_prover(
                        &client,
                        solana_deployment_root,
                    )
                    .await?;
                }
            }
        }
        Cosmwasm::AmpdSetup => {
            cmd::cosmwasm::ampd::setup_ampd(solana_deployment_root).await?;
        }
        Cosmwasm::AmpdAndTofndRun => cmd::cosmwasm::ampd::start_with_tofnd().await?,
        Cosmwasm::RedeployAndInitAll {
            axelar_private_key_hex: private_key_hex,
        } => {
            let client = create_axelar_cosmsos_signer(
                private_key_hex,
                &solana_deployment_root.axelar_configuration,
            )?;
            cmd::cosmwasm::deploy(&client, &mut solana_deployment_root.axelar_configuration)
                .await?;
            cmd::cosmwasm::init_solana_voting_verifier(&client, solana_deployment_root).await?;
            cmd::cosmwasm::init_gateway(&client, solana_deployment_root).await?;
            cmd::cosmwasm::init_solana_multisig_prover(&client, solana_deployment_root).await?;
        }
    };
    Ok(())
}

fn handle_generating_evm_wallet(axelar_deployment_root: &AxelarDeploymentRoot) -> eyre::Result<()> {
    let mnemonic = bip39::Mnemonic::generate(24)?;
    let words = mnemonic.words().collect::<Vec<_>>().join(" ");
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

    let allowed_chains = axelar_deployment_root.chains.keys().collect::<Vec<_>>();
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
    chain: &EvmChain,
    private_key_hex: String,
) -> evm_contracts_test_suite::EvmSigner {
    let private_key =
        SecretKey::from_slice(hex::decode(private_key_hex).unwrap().as_ref()).unwrap();
    let wallet = LocalWallet::from_bytes(private_key.to_bytes().as_ref()).unwrap();
    let source_signer = init_evm_signer(&chain.rpc.parse().unwrap(), wallet.clone()).await;

    evm_contracts_test_suite::EvmSigner {
        wallet: wallet.clone(),
        signer: source_signer,
    }
}

fn create_axelar_cosmsos_signer(
    axelar_private_key_hex: String,
    config: &AxelarConfiguration,
) -> Result<SigningClient, eyre::Error> {
    let key_bytes = hex::decode(axelar_private_key_hex)?;
    let signing_key = cosmrs::crypto::secp256k1::SigningKey::from_slice(&key_bytes)
        .context("invalid secp256k1 private key")?;
    let cosmwasm_signer = SigningClient {
        network: cmd::cosmwasm::cosmos_client::network::Network::new(
            config.axelar_chain.chain_id.clone(),
            config.axelar_chain.grpc.clone(),
            config.axelar_chain.rpc.clone(),
        ),
        account_prefix: config.axelar_account_prefix.clone(),
        signing_key,
    };
    Ok(cosmwasm_signer)
}

fn get_axelar_configuration(axelar_chain_name: &str) -> eyre::Result<AxelarDeploymentRoot> {
    let path = match axelar_chain_name {
        "devnet-amplifier" => xtask_crate_root_dir().join("devnet-amplifier.json"),
        _ => eyre::bail!("invalid axelar chain name"),
    };

    let file = std::fs::File::open(path)?;
    let deployment = AxelarDeploymentRoot::from_reader(file);
    Ok(deployment)
}
