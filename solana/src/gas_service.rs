use anchor_lang::ToAccountMetas;
use clap::{Parser, Subcommand};
use solana_sdk::instruction::{AccountMeta, Instruction};
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

    /// Add more native SOL gas to an existing transaction.
    AddGas(AddGasArgs),
}

#[derive(Parser, Debug)]
pub(crate) struct InitArgs {
    /// The account to set as operator of the AxelarGasService program. This account will be able
    /// to withdraw funds from the AxelarGasService program and update the configuration.
    #[clap(short, long)]
    operator: Pubkey,
}

#[derive(Parser, Debug)]
pub(crate) struct AddGasArgs {
    /// The message ID of the contract call
    #[clap(short, long)]
    message_id: String,
    /// The amount of gas to add
    #[clap(short, long)]
    amount: u64,
    /// The address to refund the gas to
    #[clap(short, long)]
    refund_address: Pubkey,
}

pub(crate) fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = match command {
        Commands::Init(init_args) => init(fee_payer, init_args, config)?,
        Commands::AddGas(add_gas_args) => add_gas(fee_payer, add_gas_args)?,
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
    let (treasury_pda, _) =
        Pubkey::find_program_address(&[b"gas-service"], &solana_axelar_gas_service::id());

    let (operator_pda, _) = Pubkey::find_program_address(
        &[b"operator", init_args.operator.as_ref()],
        &solana_axelar_operators::ID,
    );

    let mut chains_info: serde_json::Value = read_json_file_from_path(&config.chains_info_file)?;
    chains_info[CHAINS_KEY][&config.chain][CONTRACTS_KEY][GAS_SERVICE_KEY] = serde_json::json!({
        ADDRESS_KEY: solana_axelar_gas_service::id().to_string(),
        OPERATOR_KEY: init_args.operator.to_string(),
        CONFIG_ACCOUNT_KEY: treasury_pda.to_string(),
        UPGRADE_AUTHORITY_KEY: fee_payer.to_string(),
    });

    write_json_to_file_path(&chains_info, &config.chains_info_file)?;

    let ix_data = {
        use anchor_lang::InstructionData;
        solana_axelar_gas_service::instruction::Initialize {}.data()
    };

    Ok(vec![Instruction {
        program_id: solana_axelar_gas_service::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new_readonly(init_args.operator, true),
            AccountMeta::new_readonly(operator_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::ID, false),
            AccountMeta::new(treasury_pda, false),
        ],
        data: ix_data,
    }])
}

fn add_gas(fee_payer: &Pubkey, add_gas_args: AddGasArgs) -> eyre::Result<Vec<Instruction>> {
    let treasury_pda = Pubkey::find_program_address(
        &[solana_axelar_gas_service::state::Treasury::SEED_PREFIX],
        &solana_axelar_gas_service::id(),
    )
    .0;

    let (event_authority_pda, _) =
        Pubkey::find_program_address(&[b"__event_authority"], &solana_axelar_gas_service::id());

    let accounts = solana_axelar_gas_service::accounts::AddGas {
        sender: *fee_payer,
        treasury: treasury_pda,
        system_program: solana_sdk_ids::system_program::ID,
        program: solana_axelar_gas_service::id(),
        event_authority: event_authority_pda,
    }
    .to_account_metas(None);

    let ix_data = {
        use anchor_lang::InstructionData;
        solana_axelar_gas_service::instruction::AddGas {
            message_id: add_gas_args.message_id,
            amount: add_gas_args.amount,
            refund_address: add_gas_args.refund_address,
        }
        .data()
    };

    Ok(vec![Instruction {
        program_id: solana_axelar_gas_service::id(),
        accounts,
        data: ix_data,
    }])
}
