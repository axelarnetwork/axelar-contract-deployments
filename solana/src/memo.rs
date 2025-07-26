use clap::Subcommand;
use solana_sdk::instruction::Instruction;
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
    let counter_pda = axelar_solana_memo_program::get_counter_pda();

    let init_instruction =
        axelar_solana_memo_program::instruction::initialize(fee_payer, &counter_pda)?;

    println!("------------------------------------------");
    println!(
        "\u{2705} Memo program ({}) initialization details:",
        axelar_solana_memo_program::id()
    );
    println!("   Counter Account: {}", counter_pda.0);
    println!("------------------------------------------");

    Ok(vec![init_instruction])
}
