use clap::{Parser, Subcommand};
use solana_sdk::instruction::Instruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::{
    ADDRESS_KEY, CHAINS_KEY, CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, GAS_SERVICE_KEY, OPERATOR_KEY,
    UPGRADE_AUTHORITY_KEY, fetch_latest_blockhash, read_json_file_from_path,
    write_json_to_file_path,
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize the AxelarGasService program on Solana
    Init(InitArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    /// The account to set as operator of the AxelarGasService program. This account will be able
    /// to withdraw funds from the AxelarGasService program and update the configuration.
    #[clap(short, long)]
    operator: Pubkey,

    /// The salt used to derive the config PDA. This should be a unique value for each deployment.
    #[clap(short, long)]
    salt: String,
}

pub(crate) fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config)?,
    };

    // Get blockhash
    let blockhash = fetch_latest_blockhash(&config.url)?;

    // Create a transaction for each individual instruction
    let mut serializable_transactions = Vec::with_capacity(instructions.len());

    for instruction in instructions {
        // Build message and transaction with blockhash for a single instruction
        let message = solana_sdk::message::Message::new_with_blockhash(
            &[instruction],
            Some(fee_payer),
            &blockhash,
        );
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

fn init(
    fee_payer: &Pubkey,
    init_args: InitArgs,
    config: &Config,
) -> eyre::Result<Vec<Instruction>> {
    let (config_pda, _bump) = solana_axelar_gas_service::get_config_pda();

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][&config.chain][CONTRACTS_KEY][GAS_SERVICE_KEY] = serde_json::json!({
        ADDRESS_KEY: solana_axelar_gas_service::id().to_string(),
        OPERATOR_KEY: init_args.operator.to_string(),
        CONFIG_ACCOUNT_KEY: config_pda.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    Ok(vec![solana_axelar_gas_service::instructions::init_config(
        fee_payer,
        &init_args.operator,
    )?])
}
