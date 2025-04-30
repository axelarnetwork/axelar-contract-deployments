use clap::{Parser, Subcommand};
use serde::Deserialize;
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::{
    config::Config,
    types::ChainNameOnAxelar,
    utils::{self, ADDRESS_KEY, AXELAR_KEY, CONTRACTS_KEY, ITS_KEY},
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    #[clap(long_about = "Initialize the ITS program")]
    Init(InitArgs),

    #[clap(long_about = "Set the pause status of the ITS program")]
    SetPauseStatus(SetPauseStatusArgs),

    #[clap(long_about = "Add a new trusted chain to ITS")]
    SetTrustedChain(TrustedChainArgs),

    #[clap(long_about = "Remove an existing trusted chain from ITS")]
    RemoveTrustedChain(TrustedChainArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    #[clap(short, long)]
    operator: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct SetPauseStatusArgs {
    #[clap(short, long)]
    paused: bool,
}

#[derive(Parser, Debug)]
pub(crate) struct TrustedChainArgs {
    #[clap(short, long)]
    chain_name: String,
}

pub(crate) async fn build_instruction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Instruction> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await,
        Commands::SetPauseStatus(set_pause_args) => {
            set_pause_status(fee_payer, set_pause_args).await
        }
        Commands::SetTrustedChain(set_trusted_chain_args) => {
            set_trusted_chain(fee_payer, set_trusted_chain_args).await
        }
        Commands::RemoveTrustedChain(remove_trusted_chain_args) => {
            remove_trusted_chain(fee_payer, remove_trusted_chain_args).await
        }
    }
}

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Instruction> {
    let its_hub_address = String::deserialize(
        &utils::chains_info(config.network_type)[AXELAR_KEY][CONTRACTS_KEY][ITS_KEY][ADDRESS_KEY],
    )?;

    Ok(axelar_solana_its::instruction::initialize(
        *fee_payer,
        axelar_solana_gateway::get_gateway_root_config_pda().0,
        init_args.operator,
        ChainNameOnAxelar::from(config.network_type).0,
        its_hub_address,
    )?)
}

async fn set_pause_status(
    fee_payer: &Pubkey,
    set_pause_args: SetPauseStatusArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::set_pause_status(
        *fee_payer,
        set_pause_args.paused,
    )?)
}

async fn set_trusted_chain(
    fee_payer: &Pubkey,
    set_trusted_chain_args: TrustedChainArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::set_trusted_chain(
        *fee_payer,
        set_trusted_chain_args.chain_name,
    )?)
}

async fn remove_trusted_chain(
    fee_payer: &Pubkey,
    remove_trusted_chain_args: TrustedChainArgs,
) -> eyre::Result<Instruction> {
    Ok(axelar_solana_its::instruction::remove_trusted_chain(
        *fee_payer,
        remove_trusted_chain_args.chain_name,
    )?)
}
