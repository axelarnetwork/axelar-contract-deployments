use clap::{Parser, Subcommand};
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    pubkey::Pubkey,
    transaction::Transaction as SolanaTransaction
};

use crate::{
    config::Config,
    types::{ChainNameOnAxelar, SerializableSolanaTransaction, SolanaTransactionParams},
    utils::{
        fetch_latest_blockhash, read_json_file_from_path, write_json_to_file_path, ADDRESS_KEY,
        CHAINS_KEY, CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, GAS_SERVICE_KEY,
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
) -> eyre::Result<Vec<Instruction>> {
    match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await,
    }
}

pub(crate) async fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config).await?,
    };

    // Get blockhash
    let blockhash = fetch_latest_blockhash(&config.url)?;

    // Create a transaction for each individual instruction
    let mut serializable_transactions = Vec::with_capacity(instructions.len());

    for instruction in instructions {
        // Build message and transaction with blockhash for a single instruction
        let message = solana_sdk::message::Message::new_with_blockhash(&[instruction], Some(fee_payer), &blockhash);
        let transaction = SolanaTransaction::new_unsigned(message);

        // Create the transaction parameters
        // Note: Nonce account handling is done in generate_from_transactions
        // rather than here, so each transaction gets the nonce instruction prepended
        let params = SolanaTransactionParams {
            fee_payer: fee_payer.to_string(),
            recent_blockhash: Some(blockhash.to_string()),
            nonce_account: None,
            nonce_authority: None,
            blockhash_for_message: blockhash.to_string(),
        };

        // Create a serializable transaction
        let serializable_tx = SerializableSolanaTransaction::new(transaction, params);
        serializable_transactions.push(serializable_tx);
    }

    Ok(serializable_transactions)
}

async fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
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

    Ok(vec![axelar_solana_gas_service::instructions::init_config(
        &program_id,
        fee_payer,
        &init_args.authority,
        &config_pda,
        salt_hash,
    )?])
}
