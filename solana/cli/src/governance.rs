use clap::{Parser, Subcommand};
use solana_sdk::{instruction::Instruction, pubkey::Pubkey};

use crate::{
    config::Config,
    types::ChainNameOnAxelar,
    utils::{
        read_json_file_from_path, write_json_to_file_path, ADDRESS_KEY, CHAINS_KEY,
        CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, GOVERNANCE_ADDRESS_KEY, GOVERNANCE_CHAIN_KEY,
        GOVERNANCE_KEY, UPGRADE_AUTHORITY_KEY,
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
    governance_chain: String,

    #[clap(short, long)]
    governance_address: String,

    #[clap(short, long)]
    minimum_proposal_eta_delay: u32,

    #[clap(short, long)]
    operator: Pubkey,
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
    let chain_hash = solana_sdk::keccak::hashv(&[init_args.governance_chain.as_bytes()]).0;
    let address_hash = solana_sdk::keccak::hashv(&[init_args.governance_address.as_bytes()]).0;
    let (config_pda, _bump) = axelar_solana_governance::state::GovernanceConfig::pda();

    let governance_config = axelar_solana_governance::state::GovernanceConfig::new(
        chain_hash,
        address_hash,
        init_args.minimum_proposal_eta_delay,
        init_args.operator.to_bytes(),
    );

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][ChainNameOnAxelar::from(config.network_type).0][CONTRACTS_KEY]
        [GOVERNANCE_KEY] = serde_json::json!({
        ADDRESS_KEY: axelar_solana_gateway::id().to_string(),
        CONFIG_ACCOUNT_KEY: config_pda.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
        GOVERNANCE_ADDRESS_KEY: init_args.governance_address,
        GOVERNANCE_CHAIN_KEY: init_args.governance_chain,
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(
        axelar_solana_governance::instructions::builder::IxBuilder::new()
            .initialize_config(fee_payer, &config_pda, governance_config)
            .build(),
    )
}
