use clap::{Parser, Subcommand};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::config::Config;

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
    _config: &Config,
) -> eyre::Result<Instruction> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args).await,
    }
}

async fn init(fee_payer: &Pubkey, init_args: InitArgs) -> eyre::Result<Instruction> {
    let program_id = axelar_solana_gas_service::id();
    let salt_hash = solana_sdk::keccak::hashv(&[init_args.salt.as_bytes()]).0;
    let (config_pda, _bump) =
        axelar_solana_gas_service::get_config_pda(&program_id, &salt_hash, &init_args.authority);

    Ok(axelar_solana_gas_service::instructions::init_config(
        &program_id,
        fee_payer,
        &init_args.authority,
        &config_pda,
        salt_hash,
    )?)
}
