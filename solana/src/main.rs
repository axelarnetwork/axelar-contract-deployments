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
use dotenvy::dotenv;
use eyre::eyre;
use generate::GenerateArgs;
use send::{sign_and_send_transactions, SendArgs};
use sign::SignArgs;
use solana_clap_v3_utils::input_parsers::parse_url_or_moniker;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_sdk::pubkey::Pubkey;
use types::{AxelarNetwork, SerializableSolanaTransaction};

use crate::broadcast::broadcast_solana_transaction;
use crate::combine::combine_solana_signatures;
use crate::config::Config;
use crate::generate::generate_from_transactions;
use crate::misc::do_misc;
use crate::sign::sign_solana_transaction;

/// A CLI tool to generate, sign (offline/Ledger), combine, and broadcast Solana transactions
/// related to the Axelar protocol, supporting durable nonces for delayed signing scenarios.
#[derive(Parser, Debug)]
#[clap(author, version, about = "Solana Axelar CLI")]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    /// Axelar environment to use. Options: [mainnet, testnet, devnet-amplifier, local].
    #[clap(long, env = "ENV", arg_enum)]
    env: AxelarNetwork,

    /// URL for Solana's JSON RPC or moniker (or their first letter):  [mainnet-beta, testnet,
    /// devnet, localhost]". Defaults to the value set in the Solana CLI config.
    #[clap(
        short,
        long,
        env = "CLUSTER",
        value_parser = parse_url_or_moniker,
    )]
    url: Option<String>,

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

    /// Query data from Solana.
    Query(QueryCommandArgs),
}

#[derive(Parser, Debug)]
struct SendCommandArgs {
    /// Signing key identifier (path for keypair file or usb ledger). Defaults to the keypair
    /// set in the Solana CLI config.
    #[clap(long, env = "PRIVATE_KEY")]
    fee_payer: Option<String>,

    /// List of signing key identifiers (path for keypair file or usb ledger)
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
    /// Signing key identifier (path for keypair file or usb ledger)
    signer_key: String,

    /// Path to the unsigned Solana transaction JSON file (*.unsigned.json)
    #[clap(parse(from_os_str))]
    unsigned_tx_path: PathBuf,

    /// Output directory for signature files
    /// If not specified, signatures will be placed in the same directory as the unsigned transaction
    #[clap(long = "output-dir", parse(from_os_str))]
    output_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct CombineCommandArgs {
    /// Output directory for the combined signed transaction JSON
    /// If not specified, will use the same directory as the unsigned transaction
    #[clap(long = "output-dir", parse(from_os_str))]
    output_dir: Option<PathBuf>,

    /// Paths to the partial signature JSON files (*.partial.sig) to combine (provide at least one)
    #[clap(
        required = true,
        multiple_values = true,
        min_values = 1,
        parse(from_os_str)
    )]
    signature_paths: Vec<PathBuf>,

    /// Path to the original unsigned Solana transaction JSON file (*.unsigned.json)
    #[clap(parse(from_os_str))]
    unsigned_tx_path: PathBuf,
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

#[derive(Parser, Debug)]
struct QueryCommandArgs {
    #[clap(subcommand)]
    instruction: QueryInstructionSubcommand,
}

#[derive(Subcommand, Debug)]
enum QueryInstructionSubcommand {
    /// Commands to query data from the AxelarGateway program on Solana
    #[clap(subcommand)]
    Gateway(gateway::QueryCommands),
}

#[tokio::main]
async fn main() {
    let _ = dotenv().ok();

    if let Err(e) = run().await {
        eprintln!("\nError: {e:?}");
        exit(1);
    }
}

async fn run() -> eyre::Result<()> {
    let matches = Cli::command().get_matches();
    let cli = Cli::from_arg_matches(&matches)?;

    let maybe_solana_config = solana_cli_config::CONFIG_FILE
        .as_ref()
        .map(|config_file| solana_cli_config::Config::load(config_file).ok())
        .flatten();
    let url = cli
        .url
        .or_else(|| maybe_solana_config.as_ref().map(|c| c.json_rpc_url.clone()))
        .ok_or_else(|| eyre!("No URL provided and no Solana CLI config found"))?;

    let config = Config::new(url, cli.output_dir, cli.chains_info_dir, cli.env)?;

    match cli.command {
        Command::Send(args) => {
            let key_path = args
                .fee_payer
                .or_else(|| maybe_solana_config.map(|c| c.keypair_path))
                .ok_or_else(|| eyre!("No fee payer provided and no Solana CLI config found"))?;

            let fee_payer = signer_from_path(&matches, &key_path, "fee-payer", &mut None)
                .map_err(|e| eyre!("Failed to load fee payer: {e}"))?;

            let send_args = SendArgs {
                fee_payer,
                signers: args.signer_keys,
            };

            let transactions =
                build_transaction(&send_args.fee_payer.pubkey(), args.instruction, &config).await?;
            sign_and_send_transactions(send_args, &config, transactions)?;
        }
        Command::Generate(args) => {
            let output_dir = args.output_dir.unwrap_or_else(|| config.output_dir.clone());

            let gen_args = GenerateArgs {
                fee_payer: args.fee_payer,
                nonce_account: args.nonce_account,
                nonce_authority: args.nonce_authority,
                output_dir,
            };

            let transactions =
                build_transaction(&gen_args.fee_payer, args.instruction, &config).await?;
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
            do_misc(args.instruction, &config)?;
        }
        Command::Query(args) => match args.instruction {
            QueryInstructionSubcommand::Gateway(command) => {
                gateway::query(command, &config)?;
            }
        },
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
            gas_service::build_transaction(fee_payer, command, config)
        }
        InstructionSubcommand::Its(command) => its::build_transaction(fee_payer, command, config),
        InstructionSubcommand::Governance(command) => {
            governance::build_transaction(fee_payer, command, config)
        }
    }
}
