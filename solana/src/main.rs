mod broadcast;
mod combine;
mod config;
mod gas_service;
mod gateway;
mod generate;
mod governance;
mod its;
mod misc;
mod multisig_prover_types;
mod send;
mod sign;
mod types;
mod utils;

use std::path::PathBuf;
use std::process::exit;

use broadcast::BroadcastArgs;
use clap::{FromArgMatches, IntoApp, Parser, Subcommand};
use combine::CombineArgs;
use eyre::eyre;
use generate::GenerateArgs;
use send::{sign_and_send_transactions, SendArgs};
use sign::SignArgs;
use solana_clap_v3_utils::input_parsers::parse_url_or_moniker;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_sdk::pubkey::Pubkey;
use types::SerializableSolanaTransaction;

use crate::broadcast::broadcast_solana_transaction;
use crate::combine::combine_solana_signatures;
use crate::config::Config;
use crate::generate::generate_from_transactions;
use crate::misc::build_message;
use crate::sign::sign_solana_transaction;

/// A CLI tool to generate, sign (offline/Ledger), combine, and broadcast Solana transactions
/// related to the Axelar protocol, supporting durable nonces for delayed signing scenarios.
#[derive(Parser, Debug)]
#[clap(author, version, about = "Solana Axelar CLI")]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    /// URL for Solana's JSON RPC or moniker (or their first letter):  [mainnet-beta, testnet,
    /// devnet, localhost]",
    #[clap(
        short,
        long,
        env = "URL_OR_MONIKER",
        value_parser = parse_url_or_moniker,
    )]
    url: String,

    /// Directory to store output files (unsigned tx, signatures, bundles)
    #[clap(
        short = 'o',
        long = "output-dir",
        default_value = "./output",
        parse(from_os_str)
    )]
    output_dir: PathBuf,

    /// Directory containing the JSON files for Axelar chains configuration info
    /// (devnet-amplifier.json, mainnet.json, testnet.json, etc)
    #[clap(short, long, default_value = ".", parse(from_os_str), hide(true))]
    chains_info_dir: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Build and send a transaction to the Solana network.
    Send(SendCommandArgs),

    /// Generates an unsigned Solana transaction JSON file. Uses --nonce-account and
    /// --nonce-authority for durable nonces
    Generate(GenerateCommandArgs),

    /// Sign an unsigned transaction using a local keypair file or Ledger.
    Sign(SignCommandArgs),

    /// Combine multiple partial signatures into a single file.
    Combine(CombineCommandArgs),

    /// Broadcast a combined signed transaction to the Solana network.
    Broadcast(BroadcastCommandArgs),

    /// Miscellaneous utilities.
    Misc(MiscCommandArgs),
}

#[derive(Parser, Debug)]
struct SendCommandArgs {
    /// Fee Payer Pubkey (Base58 encoded string). Loads from Solana CLI config if not passed.
    #[clap(long)]
    fee_payer: Option<Pubkey>,

    /// List of signers (Base58 encoded strings). Fee payer should also be added here in case it's
    /// not the default from Solana CLI config.
    signer_keys: Vec<String>,

    #[clap(subcommand)]
    instruction: InstructionSubcommand,
}

#[derive(Parser, Debug)]
struct GenerateCommandArgs {
    /// Fee Payer Pubkey (Base58 encoded string)
    #[clap(long)]
    fee_payer: Pubkey,

    /// Nonce account Pubkey (Base58).
    #[clap(long)]
    nonce_account: Pubkey,

    /// Nonce authority Pubkey (Base58). Must sign the transaction.
    #[clap(long)]
    nonce_authority: Pubkey,

    /// Directory to store unsigned transaction files
    #[clap(long = "output-dir", parse(from_os_str))]
    output_dir: Option<PathBuf>,

    #[clap(subcommand)]
    instruction: InstructionSubcommand,
}

#[derive(Subcommand, Debug)]
enum InstructionSubcommand {
    /// Commands to interface with the AxelarGateway program on Solana
    #[clap(subcommand)]
    Gateway(gateway::Commands),

    /// Commands to interface with the AxelarGasService program on Solana
    #[clap(subcommand)]
    GasService(gas_service::Commands),

    /// Commands to interface with the InterchainTokenService program on Solana
    #[clap(subcommand)]
    Its(its::Commands),

    /// Commands to interface with the InterchainGovernance program on Solana
    #[clap(subcommand)]
    Governance(governance::Commands),
}

#[derive(Parser, Debug)]
struct SignCommandArgs {
    /// Path to the unsigned Solana transaction JSON file (*.unsigned.json)
    #[clap(parse(from_os_str))]
    unsigned_tx_path: PathBuf,

    /// Signing key identifier (path for keypair file or usb ledger)
    #[clap(long = "signer", short = 'k')]
    signer_key: String,

    /// Output directory for signature files
    /// If not specified, signatures will be placed in the same directory as the unsigned transaction
    #[clap(long = "output-dir", parse(from_os_str))]
    output_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct CombineCommandArgs {
    /// Path to the original unsigned Solana transaction JSON file (*.unsigned.json)
    #[clap(long, parse(from_os_str))]
    unsigned_tx_path: PathBuf,

    /// Paths to the partial signature JSON files (*.partial.sig) to combine (provide at least one)
    #[clap(
        long = "signatures",
        short = 's',
        required = true,
        multiple_values = true,
        min_values = 1,
        parse(from_os_str)
    )]
    signature_paths: Vec<PathBuf>,

    /// Output directory for the combined signed transaction JSON
    /// If not specified, will use the same directory as the unsigned transaction
    #[clap(long = "output-dir", parse(from_os_str))]
    output_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct BroadcastCommandArgs {
    /// Path to the combined signed Solana transaction JSON file (*.signed.json)
    #[clap(parse(from_os_str))]
    signed_tx_path: PathBuf,
}

#[derive(Parser, Debug)]
struct MiscCommandArgs {
    #[clap(subcommand)]
    instruction: misc::Commands,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("\nError: {:?}", e);
        exit(1);
    }
}

async fn run() -> eyre::Result<()> {
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&matches)?;

    let config = Config::new(cli.url, cli.output_dir, cli.chains_info_dir)?;

    // Proceed with building and potentially sending/signing/broadcasting a Solana transaction
    match cli.command {
        Command::Send(args) => {
            let mut signer_keys = args.signer_keys;
            let fee_payer = match args.fee_payer {
                Some(fee_payer) => fee_payer,
                None => {
                    let config_file = solana_cli_config::CONFIG_FILE
                        .as_ref()
                        .ok_or_else(|| eyre!("Missing Solana config file"))?;
                    let cli_config = solana_cli_config::Config::load(config_file)?;
                    let signer =
                        signer_from_path(&matches, &cli_config.keypair_path, "signer", &mut None)
                            .map_err(|e| eyre!("Failed to load fee payer: {}", e))?;

                    signer_keys.push(cli_config.keypair_path);
                    signer.pubkey()
                }
            };
            let send_args = SendArgs {
                fee_payer,
                signers: signer_keys,
            };

            // Use the transaction-based approach
            let transactions =
                build_transaction(&send_args.fee_payer, args.instruction, &config).await?;
            sign_and_send_transactions(&send_args, &config, transactions)?;
        }
        Command::Generate(args) => {
            // Determine output directory - use provided dir or default to config.output_dir
            let output_dir = args.output_dir.unwrap_or_else(|| config.output_dir.clone());

            let gen_args = GenerateArgs {
                fee_payer: args.fee_payer,
                nonce_account: args.nonce_account,
                nonce_authority: args.nonce_authority,
                output_dir,
            };

            // Use the transaction-based approach
            let transactions =
                build_transaction(&gen_args.fee_payer, args.instruction, &config).await?;
            println!("Generating transactions...");

            let filename = utils::serialized_transactions_filename_from_arg_matches(&matches);
            generate_from_transactions(&gen_args, &config, transactions, &filename)?;
        }
        Command::Sign(args) => {
            let sign_args = SignArgs {
                unsigned_tx_path: args.unsigned_tx_path,
                signer_key: args.signer_key,
                output_dir: args.output_dir,
            };
            sign_solana_transaction(&sign_args)?;
        }
        Command::Combine(args) => {
            let combine_args = CombineArgs {
                unsigned_tx_path: args.unsigned_tx_path,
                signature_paths: args.signature_paths,
                output_dir: args.output_dir,
            };
            combine_solana_signatures(&combine_args, &config)?;
        }
        Command::Broadcast(args) => {
            let broadcast_args = BroadcastArgs {
                signed_tx_path: args.signed_tx_path,
            };
            broadcast_solana_transaction(&broadcast_args, &config)?;
        }
        Command::Misc(args) => {
            let result = build_message(args.instruction)?;
            println!("{}", result);
        }
    }
    Ok(())
}

async fn build_transaction(
    fee_payer: &Pubkey,
    instruction: InstructionSubcommand,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    match instruction {
        InstructionSubcommand::Gateway(command) => {
            gateway::build_transaction(fee_payer, command, config).await
        }
        InstructionSubcommand::GasService(command) => {
            gas_service::build_transaction(fee_payer, command, config).await
        }
        InstructionSubcommand::Its(command) => {
            its::build_transaction(fee_payer, command, config).await
        }
        InstructionSubcommand::Governance(command) => {
            governance::build_transaction(fee_payer, command, config).await
        }
    }
}
