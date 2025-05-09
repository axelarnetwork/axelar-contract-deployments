use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{
    GenerateArgs, NetworkType, SerializableInstruction, SerializableSolanaTransaction, SolanaTransactionParams,
    UnsignedSolanaTransaction,
};
use crate::utils::{self, fetch_latest_blockhash, fetch_nonce_data_and_verify};
use solana_sdk::{
    hash::Hash, instruction::Instruction as SolanaInstruction, message::Message,
    pubkey::Pubkey, system_instruction, transaction::Transaction as SolanaTransaction,
};
use std::fs::File;
use std::path::Path;
use std::str::FromStr;


pub fn generate_unsigned_solana_transaction(
    args: &GenerateArgs,
    config: &Config,
    mut instructions: Vec<SolanaInstruction>,
) -> Result<()> {
    println!("Starting unsigned Solana transaction generation...");
    println!("Network Type: {:?}", config.network_type);

    println!("Fee Payer: {}", args.fee_payer);

    let mut sdk_instructions: Vec<SolanaInstruction>;
    let blockhash_for_message: Hash;
    let mut params = SolanaTransactionParams {
        fee_payer: args.fee_payer.to_string(),
        recent_blockhash: None,
        nonce_account: None,
        nonce_authority: None,
        blockhash_for_message: Hash::default().to_string(),
    };

    match (&args.nonce_account, &args.nonce_authority) {
        (Some(nonce_account), Some(nonce_authority)) => {
            println!("Using Durable Nonce flow.");
            if args.recent_blockhash.is_some() {
                println!("Warning: --recent-blockhash is ignored when using --nonce-account.");
            }

            blockhash_for_message =
                fetch_nonce_data_and_verify(&config.url, &nonce_account, &nonce_authority)?;
            println!(
                "Using Nonce (Blockhash) from account {}: {}",
                nonce_account, blockhash_for_message
            );

            params.nonce_account = Some(nonce_account.to_string());
            params.nonce_authority = Some(nonce_authority.to_string());

            let advance_nonce_ix =
                system_instruction::advance_nonce_account(&nonce_account, &nonce_authority);
            println!("Prepending AdvanceNonceAccount instruction.");

            sdk_instructions = vec![advance_nonce_ix];
            sdk_instructions.append(&mut instructions);
        }
        (None, None) => {
            println!("Using latest blockhash flow.");
            blockhash_for_message = match &args.recent_blockhash {
                Some(bh_str) => Hash::from_str(bh_str)?,
                None => fetch_latest_blockhash(&config.url)?,
            };
            println!("Using Recent Blockhash: {}", blockhash_for_message);
            params.recent_blockhash = Some(blockhash_for_message.to_string());
            sdk_instructions = instructions;
        }
        _ => {
            return Err(AppError::InconsistentState(
                "Internal Error: CLI parser should have prevented providing only one nonce argument.".to_string(),
            ));
        }
    }

    params.blockhash_for_message = blockhash_for_message.to_string();

    let message = solana_sdk::message::Message::new_with_blockhash(
        &sdk_instructions,
        Some(&args.fee_payer),
        &blockhash_for_message,
    );

    let message_bytes = message.serialize();
    let signable_message_hex = hex::encode(&message_bytes);

    // Create unsigned transaction to write to file
    let unsigned_tx = UnsignedSolanaTransaction {
        params: params.clone(),
        instructions: sdk_instructions
            .iter()
            .map(SerializableInstruction::from)
            .collect(),
        signable_message_hex,
    };

    let unsigned_tx_filename = format!("{}.unsigned.solana.json", args.output_file);
    let unsigned_tx_path = config.output_dir.join(&unsigned_tx_filename);
    utils::save_unsigned_solana_transaction(&unsigned_tx, &unsigned_tx_path)?;
    println!(
        "Unsigned Solana transaction saved to: {}",
        unsigned_tx_path.display()
    );

    if config.network_type == NetworkType::Mainnet {
        println!("Mainnet detected. Packaging dependencies for offline signing...");
        let mut files_to_include = Vec::new();
        files_to_include.push(("unsigned_tx.solana.json", unsigned_tx_path.as_path()));
        files_to_include.push(("Cargo.toml", Path::new("./Cargo.toml")));
        files_to_include.push(("Cargo.lock", Path::new("./Cargo.lock")));
        files_to_include.push(("README.md", Path::new("./README.md")));
        files_to_include.push(("src", Path::new("./src")));

        std::fs::create_dir_all(".cargo").unwrap();
        let config_toml = File::create(".cargo/config.toml").unwrap();

        let mut cmd = std::process::Command::new("cargo")
            .arg("vendor")
            .stdout(config_toml)
            .spawn()
            .unwrap();
        cmd.wait().unwrap();

        files_to_include.push(("vendor", Path::new("./vendor")));
        files_to_include.push((".cargo/config.toml", Path::new("./.cargo/config.toml")));

        println!("OK {}", line!());
        let bundle_name = format!("{}.solana.bundle", args.output_file);
        let bundle_path =
            utils::create_offline_bundle(&bundle_name, &config.output_dir, &files_to_include)
                .unwrap();
        println!(
            "Offline bundle created for Mainnet: {}",
            bundle_path.display()
        );
        println!("-> This bundle should be securely transferred to each signer's offline machine.");
    } else {
        println!("Testnet/Devnet detected. No offline dependency packaging needed.");
    }

    Ok(())
}

pub fn generate_from_transactions(
    args: &GenerateArgs,
    config: &Config,
    mut transactions: Vec<SerializableSolanaTransaction>,
) -> Result<()> {
    println!("Starting unsigned Solana transaction generation from transactions...");
    println!("Network Type: {:?}", config.network_type);
    println!("Fee Payer: {}", args.fee_payer);

    // If nonce account is provided, we need to handle it specially
    if let (Some(nonce_account), Some(nonce_authority)) = (&args.nonce_account, &args.nonce_authority) {
        println!("Using Durable Nonce flow with account: {}", nonce_account);

        // Get nonce blockhash
        let blockhash = fetch_nonce_data_and_verify(&config.url, nonce_account, nonce_authority)?;
        println!("Using Nonce (Blockhash) from account: {}", blockhash);

        // For each transaction, we need to update with the nonce information
        // and prepend the advance_nonce_account instruction
        for tx in &mut transactions {
            // Update transaction params
            tx.params.nonce_account = Some(nonce_account.to_string());
            tx.params.nonce_authority = Some(nonce_authority.to_string());
            tx.params.blockhash_for_message = blockhash.to_string();
            tx.params.recent_blockhash = None; // Not needed with nonce

            // Create advance nonce account instruction
            let advance_nonce_ix = solana_sdk::system_instruction::advance_nonce_account(
                nonce_account,
                nonce_authority
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
                    accounts: compiled_ix.accounts.iter()
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
                &blockhash
            );

            // Update the transaction with the new message
            *tx = SerializableSolanaTransaction::new(
                solana_sdk::transaction::Transaction::new_unsigned(new_message),
                tx.params.clone()
            );
        }
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

    if config.network_type == NetworkType::Mainnet {
        println!("Mainnet detected. To create offline bundle, use the 'generate' command with instructions instead.");
        println!("-> Transactions were saved as individual files that can be signed separately.");
    } else {
        println!("Testnet/Devnet detected. No offline dependency packaging needed.");
    }

    Ok(())
}