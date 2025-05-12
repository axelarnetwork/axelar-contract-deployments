use crate::config::Config;
use crate::types::{GenerateArgs, SerializableSolanaTransaction};
use crate::utils::{self, fetch_nonce_data_and_verify};

pub fn generate_from_transactions(
    args: &GenerateArgs,
    config: &Config,
    mut transactions: Vec<SerializableSolanaTransaction>,
) -> eyre::Result<()> {
    println!("Starting unsigned Solana transaction generation from transactions...");
    println!("Network Type: {:?}", config.network_type);
    println!("Fee Payer: {}", args.fee_payer);
    println!(
        "Using Durable Nonce flow with account: {}",
        args.nonce_account
    );

    // Get nonce blockhash
    let blockhash =
        fetch_nonce_data_and_verify(&config.url, &args.nonce_account, &args.nonce_authority)?;
    println!("Using Nonce (Blockhash) from account: {}", blockhash);

    // For each transaction, we need to update with the nonce information
    // and prepend the advance_nonce_account instruction
    for tx in &mut transactions {
        // Update transaction params
        tx.params.nonce_account = Some(args.nonce_account.to_string());
        tx.params.nonce_authority = Some(args.nonce_authority.to_string());
        tx.params.blockhash_for_message = blockhash.to_string();
        tx.params.recent_blockhash = None; // Not needed with nonce

        // Create advance nonce account instruction
        let advance_nonce_ix = solana_sdk::system_instruction::advance_nonce_account(
            &args.nonce_account,
            &args.nonce_authority,
        );

        // Create a new transaction with advance_nonce_account as the first instruction
        // followed by the original transaction's instructions
        let mut instructions = vec![advance_nonce_ix];

        // Add all the original instructions from the transaction
        // We need to extract them from the message
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

        // Create a new message with the combined instructions and nonce blockhash
        let new_message = solana_sdk::message::Message::new_with_blockhash(
            &instructions,
            Some(&args.fee_payer),
            &blockhash,
        );

        // Update the transaction with the new message
        *tx = SerializableSolanaTransaction::new(
            solana_sdk::transaction::Transaction::new_unsigned(new_message),
            tx.params.clone(),
        );
    }

    // Now save each transaction
    for (i, tx) in transactions.iter().enumerate() {
        // Convert the SerializableSolanaTransaction to an UnsignedSolanaTransaction
        let unsigned_tx = tx.to_unsigned();

        // Filename includes index if we have multiple transactions
        let unsigned_tx_filename = if transactions.len() > 1 {
            format!("{}.{}.unsigned.solana.json", args.output_file, i)
        } else {
            format!("{}.unsigned.solana.json", args.output_file)
        };

        let unsigned_tx_path = config.output_dir.join(&unsigned_tx_filename);
        utils::save_unsigned_solana_transaction(&unsigned_tx, &unsigned_tx_path)?;
        println!(
            "Unsigned Solana transaction {} saved to: {}",
            i + 1,
            unsigned_tx_path.display()
        );
    }

    Ok(())
}
