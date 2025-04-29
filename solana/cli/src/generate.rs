use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{
    GenerateArgs, NetworkType, SerializableInstruction, SolanaTransactionParams,
    UnsignedSolanaTransaction,
};
use crate::utils;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    account::Account, hash::Hash, instruction::Instruction as SolanaInstruction, message::Message,
    nonce::state::State as NonceState, pubkey::Pubkey, system_instruction, system_program,
};
use std::fs::File;
use std::path::Path;
use std::str::FromStr;

fn fetch_latest_blockhash(rpc_url: &str) -> Result<Hash> {
    let rpc_client = RpcClient::new(rpc_url.to_string());
    rpc_client.get_latest_blockhash().map_err(AppError::from)
}

fn fetch_nonce_data_and_verify(
    rpc_url: &str,
    nonce_account_pubkey: &Pubkey,
    expected_nonce_authority: &Pubkey,
) -> Result<Hash> {
    let rpc_client = RpcClient::new(rpc_url.to_string());
    let nonce_account: Account = rpc_client.get_account(nonce_account_pubkey)?;

    if !system_program::check_id(&nonce_account.owner) {
        return Err(AppError::InvalidInput(format!(
            "Nonce account {} is not owned by the system program ({}), owner is {}",
            nonce_account_pubkey,
            system_program::id(),
            nonce_account.owner
        )));
    }

    let (nonce_state, _size): (NonceState, usize) =
        bincode::serde::decode_from_slice(&nonce_account.data, bincode::config::legacy()).map_err(
            |e| {
                AppError::ChainError(format!(
                    "Failed to borsh deserialize nonce account state ({}): {}",
                    nonce_account_pubkey, e
                ))
            },
        )?;

    match nonce_state {
        NonceState::Initialized(data) => {
            println!("Nonce account is initialized.");
            println!(" -> Stored Nonce (Blockhash): {}", data.blockhash());
            println!(" -> Authority: {}", data.authority);
            println!(
                " -> Fee Lamports/Signature: {}",
                data.fee_calculator.lamports_per_signature
            );

            if data.authority != *expected_nonce_authority {
                return Err(AppError::InvalidInput(format!(
                    "Nonce account authority mismatch. Expected: {}, Found in account: {}",
                    expected_nonce_authority, data.authority
                )));
            }
            Ok(data.blockhash())
        }
        NonceState::Uninitialized => Err(AppError::InvalidInput(format!(
            "Nonce account {} is uninitialized",
            nonce_account_pubkey
        ))),
    }
}

pub fn generate_unsigned_solana_transaction(
    args: &GenerateArgs,
    config: &Config,
    instruction: SolanaInstruction,
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
            sdk_instructions.push(instruction);
        }
        (None, None) => {
            println!("Using latest blockhash flow.");
            blockhash_for_message = match &args.recent_blockhash {
                Some(bh_str) => Hash::from_str(bh_str)?,
                None => fetch_latest_blockhash(&config.url)?,
            };
            println!("Using Recent Blockhash: {}", blockhash_for_message);
            params.recent_blockhash = Some(blockhash_for_message.to_string());
            sdk_instructions = vec![instruction];
        }
        _ => {
            return Err(AppError::InconsistentState(
                "Internal Error: CLI parser should have prevented providing only one nonce argument.".to_string(),
            ));
        }
    }

    params.blockhash_for_message = blockhash_for_message.to_string();

    let message = Message::new_with_blockhash(
        &sdk_instructions,
        Some(&args.fee_payer),
        &blockhash_for_message,
    );

    let message_bytes = message.serialize();
    let signable_message_hex = hex::encode(&message_bytes);
    let unsigned_tx = UnsignedSolanaTransaction {
        params,
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
