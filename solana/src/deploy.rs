use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;
use eyre::Result;
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signer, read_keypair_file};

use crate::artifact;
use crate::types::Programs;
use crate::utils::{
    GAS_SERVICE_KEY, GATEWAY_KEY, GOVERNANCE_KEY, ITS_KEY, MULTICALL_KEY, OPERATORS_KEY,
    read_json_file_from_path, try_infer_program_id_from_env,
};

#[derive(Args, Debug)]
pub(crate) struct DeployArgs {
    /// Name of the program to deploy
    #[clap(long, value_enum)]
    program: Programs,

    /// Path to the program keypair (determines program address)
    #[clap(long, env = "PROGRAM_KEYPAIR_PATH")]
    program_keypair: String,

    /// Path to the upgrade authority keypair
    #[clap(long, env = "UPGRADE_AUTHORITY_KEYPAIR_PATH")]
    upgrade_authority: String,

    /// Path to the fee payer keypair. Defaults to the Solana CLI default keypair.
    #[clap(long, env = "FEE_PAYER_KEYPAIR_PATH")]
    fee_payer: Option<String>,

    /// Skip confirmation prompt
    #[clap(short = 'y', long)]
    yes: bool,

    /// Path to the program bytecode (.so file)
    #[clap(long, conflicts_with_all = &["version", "artifact-dir"])]
    program_path: Option<String>,

    /// Version to download: semver (e.g., 0.1.7) from GitHub, or commit hash (e.g., 12e6126) from R2
    #[clap(long, conflicts_with_all = &["program-path", "artifact-dir"])]
    version: Option<String>,

    /// Directory containing local builds (e.g., ./target/deploy)
    #[clap(long, conflicts_with_all = &["program-path", "version"])]
    artifact_dir: Option<PathBuf>,
}

pub(crate) async fn deploy_program(args: DeployArgs, config: crate::Config) -> Result<()> {
    let program_path = artifact::resolve_program_path(
        &args.program,
        args.program_path.as_deref(),
        args.version.as_deref(),
        args.artifact_dir.as_deref(),
    )
    .await?;

    let fee_payer_path = get_fee_payer_path(args.fee_payer.as_deref())?;
    let upgrade_authority_pubkey = get_pubkey_from_keypair(&args.upgrade_authority)?;

    print_fee_payer_info(&fee_payer_path, &config.url)?;
    println!("Upgrade authority: {upgrade_authority_pubkey}");
    println!(
        "Deploying program {:?} using keypair {}",
        args.program, args.program_keypair
    );

    if !args.yes && !confirm_action()? {
        println!("Aborted.");
        return Ok(());
    }

    let status = Command::new("solana")
        .arg("program")
        .arg("deploy")
        .arg("--program-id")
        .arg(&args.program_keypair)
        .arg("--keypair")
        .arg(&fee_payer_path)
        .arg("--upgrade-authority")
        .arg(&args.upgrade_authority)
        .arg(program_path.to_string_lossy().as_ref())
        .status()?;

    if !status.success() {
        return Err(eyre::eyre!("solana program deploy failed"));
    }

    println!("Program {:?} deployed successfully.", args.program);
    Ok(())
}

#[derive(Args, Debug)]
pub(crate) struct UpgradeArgs {
    /// Name of the program to upgrade
    #[clap(long, value_enum)]
    program: Programs,

    /// Path to the upgrade authority keypair
    #[clap(long, env = "UPGRADE_AUTHORITY_KEYPAIR_PATH")]
    upgrade_authority: String,

    /// Path to the fee payer keypair. Defaults to the Solana CLI default keypair.
    #[clap(long, env = "FEE_PAYER_KEYPAIR_PATH")]
    fee_payer: Option<String>,

    /// Skip confirmation prompt
    #[clap(short = 'y', long)]
    yes: bool,

    /// Path to the program bytecode (.so file)
    #[clap(long, conflicts_with_all = &["version", "artifact-dir"])]
    program_path: Option<String>,

    /// Version to download: semver (e.g., 0.1.7) from GitHub, or commit hash (e.g., 12e6126) from R2
    #[clap(long, conflicts_with_all = &["program-path", "artifact-dir"])]
    version: Option<String>,

    /// Directory containing local builds (e.g., ./target/deploy)
    #[clap(long, conflicts_with_all = &["program-path", "version"])]
    artifact_dir: Option<PathBuf>,
}

pub(crate) async fn upgrade_program(args: UpgradeArgs, config: crate::Config) -> Result<()> {
    let program_path = artifact::resolve_program_path(
        &args.program,
        args.program_path.as_deref(),
        args.version.as_deref(),
        args.artifact_dir.as_deref(),
    )
    .await?;

    let env: Value = read_json_file_from_path(&config.chains_info_file)?;
    let chain = &config.chain;

    let program_key = match args.program {
        Programs::Gateway => GATEWAY_KEY,
        Programs::GasService => GAS_SERVICE_KEY,
        Programs::Governance => GOVERNANCE_KEY,
        Programs::Its => ITS_KEY,
        Programs::Multicall => MULTICALL_KEY,
        Programs::Operators => OPERATORS_KEY,
    };

    let program_id = try_infer_program_id_from_env(&env, chain, program_key)?;

    let fee_payer_path = get_fee_payer_path(args.fee_payer.as_deref())?;
    let upgrade_authority_pubkey = get_pubkey_from_keypair(&args.upgrade_authority)?;

    print_fee_payer_info(&fee_payer_path, &config.url)?;
    println!("Upgrade authority: {upgrade_authority_pubkey}");
    println!(
        "Upgrading program {:?} with ID {}",
        args.program, program_id
    );

    if !args.yes && !confirm_action()? {
        println!("Aborted.");
        return Ok(());
    }

    // Build the solana program deploy command
    let status = Command::new("solana")
        .arg("program")
        .arg("deploy")
        .arg("--program-id")
        .arg(program_id.to_string())
        .arg("--keypair")
        .arg(&fee_payer_path)
        .arg("--upgrade-authority")
        .arg(&args.upgrade_authority)
        .arg(program_path.to_string_lossy().as_ref())
        .status()?;

    if !status.success() {
        return Err(eyre::eyre!("solana program upgrade failed"));
    }

    println!("Program {:?} upgraded successfully.", args.program);
    Ok(())
}

/// Get fee payer keypair path. If not provided, uses the default Solana CLI keypair.
fn get_fee_payer_path(fee_payer: Option<&str>) -> Result<String> {
    if let Some(path) = fee_payer {
        return Ok(path.to_owned());
    }

    // Get default keypair path from solana config
    let solana_config = solana_cli_config::CONFIG_FILE
        .as_ref()
        .and_then(|config_file| solana_cli_config::Config::load(config_file).ok())
        .ok_or_else(|| eyre::eyre!("No fee payer provided and no Solana CLI config found"))?;

    Ok(solana_config.keypair_path)
}

/// Get the public key from a keypair file path.
fn get_pubkey_from_keypair(keypair_path: &str) -> Result<Pubkey> {
    let keypair = read_keypair_file(keypair_path)
        .map_err(|e| eyre::eyre!("Failed to read keypair from {}: {}", keypair_path, e))?;
    Ok(keypair.pubkey())
}

/// Print fee payer information (address and balance).
fn print_fee_payer_info(fee_payer_path: &str, rpc_url: &str) -> Result<()> {
    let path = Path::new(fee_payer_path);

    if !path.exists() {
        eprintln!("WARNING: Fee payer keypair file does not exist: {fee_payer_path}");
        return Ok(());
    }

    let pubkey = get_pubkey_from_keypair(fee_payer_path)?;
    println!("Fee payer: {pubkey}");

    // Get balance using solana CLI
    let output = Command::new("solana")
        .arg("balance")
        .arg(pubkey.to_string())
        .arg("--url")
        .arg(rpc_url)
        .output()?;

    if output.status.success() {
        let balance = String::from_utf8_lossy(&output.stdout);
        println!("Fee payer balance: {}", balance.trim());
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "WARNING: Could not fetch fee payer balance: {}",
            stderr.trim()
        );
    }

    Ok(())
}

/// Prompt the user for confirmation. Returns true if they confirm.
fn confirm_action() -> Result<bool> {
    print!("\nProceed? [y/N] ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let input = input.trim().to_lowercase();
    Ok(input == "y" || input == "yes")
}
