use anchor_lang::InstructionData;
use clap::Subcommand;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

use crate::config::Config;
use crate::types::{SerializableSolanaTransaction, SolanaTransactionParams};
use crate::utils::fetch_latest_blockhash;

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize the AxelarMemo program on Solana
    Init,
}

pub(crate) fn build_transaction(
    fee_payer: &Pubkey,
    command: Commands,
    config: &Config,
) -> eyre::Result<Vec<SerializableSolanaTransaction>> {
    let instructions = match command {
        Commands::Init => init(fee_payer, config)?,
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

fn init(fee_payer: &Pubkey, _config: &Config) -> eyre::Result<Vec<Instruction>> {
    let (counter_pda, _) = Pubkey::find_program_address(&[b"counter"], &solana_axelar_memo::id());

    let ix_data = solana_axelar_memo::instruction::Init {}.data();

    println!("------------------------------------------");
    println!(
        "\u{2705} Memo program ({}) initialization details:",
        solana_axelar_memo::id()
    );
    println!("   Counter Account: {counter_pda}");
    println!("------------------------------------------");

    Ok(vec![Instruction {
        program_id: solana_axelar_memo::id(),
        accounts: vec![
            AccountMeta::new(*fee_payer, true),
            AccountMeta::new(counter_pda, false),
            AccountMeta::new_readonly(solana_sdk::system_program::id(), false),
        ],
        data: ix_data,
    }])
}
