mod broadcast;
mod combine;
mod config;
mod error;
mod gas_service;
mod gateway;
mod generate;
mod governance;
mod its;
mod send;
mod sign;
mod types;
mod utils;

use clap::{ArgGroup, Parser, Subcommand};
use send::build_and_send_solana_transaction;
use solana_clap_v3_utils::input_parsers::parse_url_or_moniker;
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use std::path::PathBuf;
use std::process::exit;
use types::SendArgs;

use crate::broadcast::broadcast_solana_transaction;
use crate::combine::combine_solana_signatures;
use crate::config::Config;
use crate::generate::generate_unsigned_solana_transaction;
use crate::sign::sign_solana_transaction;
use crate::types::{BroadcastArgs, CombineArgs, GenerateArgs, SignArgs};

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Solana Key Management Tool for Offline/Multisig Workflows",
    long_about = "A CLI tool to generate, sign (offline/Ledger), combine, and broadcast Solana transactions, supporting durable nonces for delayed signing scenarios."
)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[clap(
        short,
        long,
        env = "URL_OR_MONIKER",
        value_parser = parse_url_or_moniker,
        help = "URL for Solana's JSON RPC or moniker (or their first letter): \
                [mainnet-beta, testnet, devnet, localhost]",
    )]
    url: String,

    /// Directory to store output files (unsigned tx, signatures, bundles)
    #[clap(
        short = 'o',
        long = "output-dir",
        default_value = "./output",
        parse(from_os_str),
        help = "Directory for output files"
    )]
    output_dir: PathBuf,

    /// Directory containing the JSON files for Axelar chains configuration info
    /// (devnet-amplifier.json, mainnet.json, testnet.json, etc)
    #[clap(short, long, default_value = ".", parse(from_os_str))]
    chains_info_dir: PathBuf,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[clap(long_about = "")]
    Send(SendCommandArgs),

    #[clap(long_about = "Generates an unsigned Solana transaction JSON file. \
    Use --nonce-account and --nonce-authority for durable nonces, otherwise fetches the latest blockhash. \
    On mainnet, also creates a *.tar.gz bundle for offline signing.")]
    Generate(GenerateCommandArgs),

    /// Sign an unsigned transaction using a local keypair file or Ledger.
    #[clap(
        long_about = "Signs the message bytes from an unsigned transaction JSON file. \
    Requires a keypair file path (for testnet/devnet) or the signer's public key (for mainnet/Ledger). \
    Outputs a partial signature JSON file."
    )]
    Sign(SignCommandArgs),

    /// Combine multiple partial signatures into a single file.
    #[clap(
        long_about = "Combines multiple partial signature JSON files corresponding to an unsigned transaction. \
    Validates that all required signers (including fee payer and nonce authority if applicable) have provided signatures. \
    Outputs a combined signed transaction JSON file."
    )]
    Combine(CombineCommandArgs),

    /// Broadcast a combined signed transaction to the Solana network.
    #[clap(
        long_about = "Reconstructs a Solana transaction from a combined signed transaction JSON file \
    and broadcasts it to the specified network via RPC. Waits for confirmation."
    )]
    Broadcast(BroadcastCommandArgs),
}

#[derive(Parser, Debug)]
struct SendCommandArgs {
    /// Fee Payer Pubkey (Base58 encoded string)
    #[clap(long)]
    fee_payer: Pubkey,

    #[clap(long, help = "List of signers (Base58 encoded strings)")]
    signer_keys: Vec<String>,

    #[clap(subcommand)]
    instruction: InstructionSubcommand,
}

#[derive(Parser, Debug)]
#[clap(group(ArgGroup::new("blockhash_source").required(false).args(&["nonce-account", "recent-blockhash"])))]
struct GenerateCommandArgs {
    /// Fee Payer Pubkey (Base58 encoded string)
    #[clap(long)]
    fee_payer: Pubkey,

    /// Nonce account Pubkey (Base58). Requires --nonce-authority.
    #[clap(long, requires = "nonce-authority")]
    nonce_account: Option<Pubkey>,

    /// Nonce authority Pubkey (Base58). Must sign the transaction.
    #[clap(long, requires = "nonce-account")]
    nonce_authority: Option<Pubkey>,

    /// Specify a recent blockhash (Base58) instead of fetching or using a nonce.
    #[clap(long, global = true)]
    recent_blockhash: Option<String>,

    /// Base name for output files (e.g., 'my_tx' -> my_tx.unsigned.solana.json)
    #[clap(long = "output-name")]
    output_file: String,

    #[clap(subcommand)]
    instruction: InstructionSubcommand,
}

#[derive(Subcommand, Debug)]
enum InstructionSubcommand {
    #[clap(long_about = "Commands for Gateway program", subcommand)]
    Gateway(gateway::Commands),

    #[clap(long_about = "Commands for GasService program", subcommand)]
    GasService(gas_service::Commands),

    #[clap(long_about = "Commands for InterchainTokenService program", subcommand)]
    Its(its::Commands),

    #[clap(long_about = "Commands for Governance program", subcommand)]
    Governance(governance::Commands),
}

#[derive(Parser, Debug)]
struct SignCommandArgs {
    /// Path to the unsigned Solana transaction JSON file (*.unsigned.solana.json)
    #[clap(parse(from_os_str))]
    unsigned_tx_path: PathBuf,

    /// Signing key identifier (path for keypair file or usb ledger)
    #[clap(long = "signer", short = 'k')]
    signer_key: String,

    /// Output file path for the generated partial signature JSON (*.sig.json)
    #[clap(long = "output-sig", short = 's', parse(from_os_str))]
    // Changed from arg(value_parser = ...)
    output_signature_path: PathBuf,
}

#[derive(Parser, Debug)]
struct CombineCommandArgs {
    /// Path to the original unsigned Solana transaction JSON file (*.unsigned.solana.json)
    #[clap(long, parse(from_os_str))]
    unsigned_tx_path: PathBuf,

    /// Paths to the partial signature JSON files (*.sig.json) to combine (provide at least one)
    #[clap(
        long = "signatures",
        short = 's',
        required = true,
        multiple_values = true,
        min_values = 1,
        parse(from_os_str)
    )]
    signature_paths: Vec<PathBuf>,

    /// Output file path for the combined signed transaction JSON (*.signed.solana.json)
    #[clap(long = "output-signed", short = 'f', parse(from_os_str))]
    output_signed_tx_path: PathBuf,
}

#[derive(Parser, Debug)]
struct BroadcastCommandArgs {
    /// Path to the combined signed Solana transaction JSON file (*.signed.solana.json)
    #[clap(parse(from_os_str))]
    signed_tx_path: PathBuf,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("\nError: {}", e);
        exit(1);
    }
}

async fn run() -> eyre::Result<()> {
    let cli = Cli::parse();

    let config = Config::new(cli.url, cli.output_dir, cli.chains_info_dir)?;

    // Proceed with building and potentially sending/signing/broadcasting a Solana transaction
    match cli.command {
        Command::Send(args) => {
            let send_args = SendArgs {
                fee_payer: args.fee_payer,
                signers: args.signer_keys,
            };
            let instruction =
                build_instruction(&send_args.fee_payer, args.instruction, &config).await?;
            build_and_send_solana_transaction(&send_args, &config, instruction)?;
        }
        Command::Generate(args) => {
            let gen_args = GenerateArgs {
                fee_payer: args.fee_payer,
                nonce_account: args.nonce_account,
                nonce_authority: args.nonce_authority,
                recent_blockhash: args.recent_blockhash,
                output_file: args.output_file,
            };
            let instruction =
                build_instruction(&gen_args.fee_payer, args.instruction, &config).await?;
            println!("Generating...");
            generate_unsigned_solana_transaction(&gen_args, &config, instruction)?;
        }
        Command::Sign(args) => {
            let sign_args = SignArgs {
                unsigned_tx_path: args.unsigned_tx_path,
                signer_key: args.signer_key,
                output_signature_path: args.output_signature_path,
            };
            sign_solana_transaction(&sign_args)?;
        }
        Command::Combine(args) => {
            let combine_args = CombineArgs {
                unsigned_tx_path: args.unsigned_tx_path,
                signature_paths: args.signature_paths,
                output_signed_tx_path: args.output_signed_tx_path,
            };
            combine_solana_signatures(&combine_args, &config)?;
        }
        Command::Broadcast(args) => {
            let broadcast_args = BroadcastArgs {
                signed_tx_path: args.signed_tx_path,
            };
            broadcast_solana_transaction(&broadcast_args, &config)?;
        }
    }
    Ok(())
}

async fn build_instruction(
    fee_payer: &Pubkey,
    instruction: InstructionSubcommand,
    config: &Config,
) -> eyre::Result<Instruction> {
    let serializable_ix = match instruction {
        InstructionSubcommand::Gateway(command) => {
            gateway::build_instruction(fee_payer, command, config).await?
        }
        InstructionSubcommand::GasService(command) => {
            gas_service::build_instruction(fee_payer, command, config).await?
        }
        InstructionSubcommand::Its(command) => {
            its::build_instruction(fee_payer, command, config).await?
        }
        InstructionSubcommand::Governance(command) => {
            governance::build_instruction(fee_payer, command, config).await?
        }
    };

    Ok(serializable_ix)
}
