use solana_sdk::pubkey::Pubkey;

use crate::config::Config;
use crate::types::SerializableSolanaTransaction;
use crate::utils::{self, fetch_nonce_data_and_verify};

#[derive(Debug, Clone)]
pub(crate) struct GenerateArgs {
    pub(crate) fee_payer: Pubkey,
    pub(crate) nonce_account: Pubkey,
    pub(crate) nonce_authority: Pubkey,
    pub(crate) output_dir: std::path::PathBuf,
}

pub(crate) fn generate_from_transactions(
    args: &GenerateArgs,
    config: &Config,
    mut transactions: Vec<SerializableSolanaTransaction>,
    filename: &str,
) -> eyre::Result<()> {
    println!("Starting unsigned Solana transaction generation from transactions...");
    println!("Network Type: {:?}", config.network_type);
    println!("Fee Payer: {}", args.fee_payer);
    println!(
        "Using Durable Nonce flow with account: {}",
        args.nonce_account
    );

    let blockhash =
        fetch_nonce_data_and_verify(&config.url, &args.nonce_account, &args.nonce_authority)?;
    println!("Using Nonce (Blockhash) from account: {blockhash}");

    for tx in &mut transactions {
        tx.params.nonce_account = Some(args.nonce_account.to_string());
        tx.params.nonce_authority = Some(args.nonce_authority.to_string());
        tx.params.blockhash_for_message = blockhash.to_string();
        tx.params.recent_blockhash = None;

        let advance_nonce_ix = solana_sdk::system_instruction::advance_nonce_account(
            &args.nonce_account,
            &args.nonce_authority,
        );
        let mut instructions = vec![advance_nonce_ix];

        let original_message = tx.transaction.message.clone();
        let account_keys = original_message.account_keys.clone();

        for compiled_ix in &original_message.instructions {
            let ix = solana_sdk::instruction::Instruction {
                program_id: account_keys[compiled_ix.program_id_index as usize],
                accounts: compiled_ix
                    .accounts
                    .iter()
                    .map(|idx| {
                        let pubkey = account_keys[*idx as usize];
                        solana_sdk::instruction::AccountMeta {
                            pubkey,
                            is_signer: original_message.is_signer(*idx as usize),
                            is_writable: original_message.is_maybe_writable(*idx as usize, None),
                        }
                    })
                    .collect(),
                data: compiled_ix.data.clone(),
            };

            instructions.push(ix);
        }

        let new_message = solana_sdk::message::Message::new_with_blockhash(
            &instructions,
            Some(&args.fee_payer),
            &blockhash,
        );

        *tx = SerializableSolanaTransaction::new(
            solana_sdk::transaction::Transaction::new_unsigned(new_message),
            tx.params.clone(),
        );
    }

    std::fs::create_dir_all(&args.output_dir)?;

    for (i, tx) in transactions.iter().enumerate() {
        let unsigned_tx = tx.to_unsigned();

        let unsigned_tx_filename = if transactions.len() > 1 {
            format!("{filename}.{i}.unsigned.json")
        } else {
            format!("{filename}.unsigned.json")
        };

        let unsigned_tx_path = args.output_dir.join(&unsigned_tx_filename);
        utils::save_unsigned_solana_transaction(&unsigned_tx, &unsigned_tx_path)?;
        println!(
            "Unsigned Solana transaction {} saved to: {}",
            i + 1,
            unsigned_tx_path.display()
        );
    }

    Ok(())
}
