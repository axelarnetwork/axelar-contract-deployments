use clap::Args;
use serde_json::Value;
use std::process::Command;

use crate::{
    types::Programs,
    utils::{
        GAS_SERVICE_KEY, GATEWAY_KEY, GOVERNANCE_KEY, ITS_KEY, MULTICALL_KEY,
        read_json_file_from_path, try_infer_program_id_from_env,
    },
};

#[derive(Args, Debug)]
pub(crate) struct UpgradeArgs {
    /// Name of the program to deploy
    #[clap(long, value_enum)]
    program: Programs,

    /// Path to the upgrade authority keypair
    #[clap(long, env = "UPGRADE_AUTHORITY_KEYPAIR_PATH")]
    upgrade_authority: String,

    /// Path to the program bytecode (.so file)
    #[clap(long)]
    program_path: String,
}

pub(crate) fn upgrade_program(args: UpgradeArgs, config: crate::Config) -> eyre::Result<()> {
    // Read the environment JSON file
    let env: Value = read_json_file_from_path(&config.chains_info_file)?;
    let chain = &config.chain;

    let program_key = match args.program {
        Programs::Gateway => GATEWAY_KEY,
        Programs::GasService => GAS_SERVICE_KEY,
        Programs::Governance => GOVERNANCE_KEY,
        Programs::Its => ITS_KEY,
        Programs::Multicall => MULTICALL_KEY,
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
        .arg(&args.program_path)
        .status()?;

    if !status.success() {
        return Err(eyre::eyre!("solana program upgrade failed"));
    }
    println!("Program {:?} upgraded successfully.", args.program);
    Ok(())
}
