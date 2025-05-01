use clap::{Parser, Subcommand};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::{
    config::Config,
    types::ChainNameOnAxelar,
    utils::{
        read_json_file_from_path, write_json_to_file_path, ADDRESS_KEY, CHAINS_KEY,
        CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, GAS_SERVICE_KEY,
    },
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    #[clap(long_about = "Initialize the Gateway program")]
    Init(InitArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    #[clap(short, long)]
    authority: Pubkey,

    #[clap(short, long)]
    salt: String,
}

pub(crate) async fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Instruction> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await,
    }
}

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let program_id = axelar_solana_gas_service::id();
    let salt_hash = solana_sdk::keccak::hashv(&[init_args.salt.as_bytes()]).0;
    let (config_pda, _bump) =
        axelar_solana_gas_service::get_config_pda(&program_id, &salt_hash, &init_args.authority);

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
        [GAS_SERVICE_KEY] = serde_json::json!({
        ADDRESS_KEY: axelar_solana_gateway::id().to_string(),
        CONFIG_ACCOUNT_KEY: config_pda.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(axelar_solana_gas_service::instructions::init_config(
        &program_id,
        fee_payer,
        &init_args.authority,
        &config_pda,
        salt_hash,
    )?)
}
