use clap::{Parser, Subcommand};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::{
    ADDRESS_KEY, CHAINS_KEY, CONFIG_ACCOUNT_KEY, CONTRACTS_KEY, OPERATORS_KEY, OWNER_KEY,
    UPGRADE_AUTHORITY_KEY, fetch_latest_blockhash, read_json_file_from_path,
    write_json_to_file_path,
};

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    Init(InitArgs),
    AddOperator(AddOperatorArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    #[clap(short, long)]
    owner: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct AddOperatorArgs {
    #[clap(short, long)]
    operator: Pubkey,
}

pub(crate) fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config)?,
        Commands::AddOperator(add_operator_args) => add_operator(fee_payer, add_operator_args)?,
    };

    let blockhash = fetch_latest_blockhash(&config.url)?;

    let mut serializable_transactions = Vec::with_capacity(instructions.len());

    for instruction in instructions {
        let message = solana_sdk::message::Message::new_with_blockhash(
            &[instruction],
            Some(fee_payer),
            &blockhash,
        );
        let transaction = SolanaTransaction::new_unsigned(message);

        let params = SolanaTransactionParams {
            fee_payer: fee_payer.to_string(),
            recent_blockhash: Some(blockhash.to_string()),
            nonce_account: None,
            nonce_authority: None,
            blockhash_for_message: blockhash.to_string(),
        };

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
    let (registry_pda, _) =
        Pubkey::find_program_address(&[b"operator_registry"], &solana_axelar_operators::ID);

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][&config.chain][CONTRACTS_KEY][OPERATORS_KEY] = serde_json::json!({
        ADDRESS_KEY: solana_axelar_operators::id().to_string(),
        CONFIG_ACCOUNT_KEY: registry_pda.to_string(),
        OWNER_KEY: init_args.owner.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    let ix_data = {
        use anchor_lang::InstructionData;
        solana_axelar_operators::instruction::Initialize {}.data()
    };

    Ok(vec![Instruction {
        program_id: solana_axelar_operators::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(init_args.owner, false),
            AccountMeta::new(registry_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: ix_data,
    }])
}

fn add_operator(
    fee_payer: &Pubkey,
    add_operator_args: AddOperatorArgs,
) -> eyre::Result<Vec<Instruction>> {
    let (registry_pda, _) =
        Pubkey::find_program_address(&[b"operator_registry"], &solana_axelar_operators::ID);

    let (operator_pda, _) = Pubkey::find_program_address(
        &[b"operator", add_operator_args.operator.as_ref()],
        &solana_axelar_operators::ID,
    );

    let ix_data = {
        use anchor_lang::InstructionData;
        solana_axelar_operators::instruction::AddOperator {}.data()
    };

    Ok(vec![Instruction {
        program_id: solana_axelar_operators::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(add_operator_args.operator, false),
            AccountMeta::new(registry_pda, false),
            AccountMeta::new(operator_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: ix_data,
    }])
}
