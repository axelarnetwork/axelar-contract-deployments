use std::path::PathBuf;
use std::process::Command;

use clap::Args;
use eyre::Result;
use serde_json::Value;

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

pub(crate) async fn deploy_program(args: DeployArgs, _config: crate::Config) -> Result<()> {
    let program_path = artifact::resolve_program_path(
        &args.program,
        &args.program_path,
        &args.version,
        &args.artifact_dir,
    )
    .await?;

    println!(
        "Deploying program {:?} using keypair {} with authority {}",
        args.program, args.program_keypair, args.upgrade_authority
    );

    let status = Command::new("solana")
        .arg("program")
        .arg("deploy")
        .arg("--program-id")
        .arg(&args.program_keypair)
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
        &args.program_path,
        &args.version,
        &args.artifact_dir,
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

    println!(
        "Upgrading program {:?} with ID {} using authority {}",
        args.program, program_id, args.upgrade_authority
    );

    // Build the solana program deploy command
    let status = Command::new("solana")
        .arg("program")
        .arg("deploy")
        .arg("--program-id")
        .arg(program_id.to_string())
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
