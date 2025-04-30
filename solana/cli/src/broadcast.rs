use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{BroadcastArgs, SignedSolanaTransaction};
use crate::utils::{self, print_transaction_result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, hash::Hash, instruction::Instruction as SolanaInstruction,
    message::Message, pubkey::Pubkey, signature::Signature, transaction::Transaction,
};
use std::collections::HashMap;
use std::str::FromStr;

fn submit_solana_transaction(
    url: &str,
    signed_tx_data: &SignedSolanaTransaction,
) -> Result<Signature> {
    println!(
        "Reconstructing Solana transaction for broadcasting via RPC: {}",
        url
    );

    let fee_payer = Pubkey::from_str(&signed_tx_data.unsigned_tx_data.params.fee_payer)?;
    let recent_blockhash =
        Hash::from_str(&signed_tx_data.unsigned_tx_data.params.blockhash_for_message)?;
    let sdk_instructions: Vec<SolanaInstruction> = signed_tx_data
        .unsigned_tx_data
        .instructions
        .iter()
        .map(SolanaInstruction::try_from)
        .collect::<Result<Vec<_>>>()?;

    let message = Message::new(&sdk_instructions, Some(&fee_payer));

    let mut signatures_map: HashMap<Pubkey, Signature> = signed_tx_data
        .signatures
        .iter()
        .map(|ps| {
            Ok((
                Pubkey::from_str(&ps.signer_pubkey)?,
                Signature::from_str(&ps.signature)?,
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    let mut ordered_signatures: Vec<Signature> =
        Vec::with_capacity(message.header.num_required_signatures as usize);
    let mut missing_sig_for_required_signer = false;
    for (index, key) in message.account_keys.iter().enumerate() {
        if message.is_signer(index) {
            match signatures_map.remove(key) {
                Some(signature) => {
                    ordered_signatures.push(signature);
                }
                None => {
                    eprintln!(
                        "Critical Error during broadcast reconstruction: Missing signature for required signer {} (index {}).",
                        key, index
                    );
                    ordered_signatures.push(Signature::default());
                    missing_sig_for_required_signer = true;
                }
            }
        }
    }

    if missing_sig_for_required_signer {
        return Err(AppError::BroadcastError(
            "Cannot broadcast: Missing signature for one or more required signers during final reconstruction.".to_string()
        ));
    }

    if !signatures_map.is_empty() {
        println!(
            "Warning: The following signatures were provided but not required by the transaction message: {:?}",
            signatures_map
                .keys()
                .map(|pk| pk.to_string())
                .collect::<Vec<_>>()
        );
    }

    let mut transaction = Transaction::new_unsigned(message);

    if ordered_signatures.len() != transaction.signatures.len() {
        return Err(AppError::InconsistentState(format!(
            "Signature count mismatch during reconstruction: Expected {} based on message header, but gathered {}.",
            transaction.signatures.len(),
            ordered_signatures.len()
        )));
    }
    transaction.signatures = ordered_signatures;
    transaction.message.recent_blockhash = recent_blockhash;

    println!(
        "Transaction reconstructed with blockhash: {}",
        recent_blockhash
    );

    if let Err(e) = transaction.verify() {
        return Err(AppError::BroadcastError(format!(
            "Constructed transaction failed structural verification: {}",
            e
        )));
    }

    println!("Connecting to RPC client at {}", url);
    let rpc_client = RpcClient::new_with_commitment(url.to_string(), CommitmentConfig::confirmed());

    println!("Broadcasting transaction...");
    match rpc_client.send_and_confirm_transaction_with_spinner(&transaction) {
        Ok(tx_signature) => {
            println!("Transaction broadcast and confirmed!");
            Ok(tx_signature)
        }
        Err(client_err) => {
            eprintln!("Error during RPC broadcast/confirmation: {}", client_err);
            if let solana_client::client_error::ClientErrorKind::RpcError(
                solana_client::rpc_request::RpcError::RpcResponseError { data: solana_client::rpc_request::RpcResponseErrorData::SendTransactionPreflightFailure(sim_result), .. }
            ) = client_err.kind() {
                 eprintln!(" -> Preflight Simulation Failure Result: {:?}", sim_result);
            } else if let solana_client::client_error::ClientErrorKind::TransactionError(tx_err) = client_err.kind() {
                 eprintln!(" -> Transaction Error Detail: {:?}", tx_err);
            }
            Err(AppError::from(client_err))
        }
    }
}

pub fn broadcast_solana_transaction(args: &BroadcastArgs, config: &Config) -> Result<()> {
    println!("Starting Solana transaction broadcast...");

    let signed_tx_data = utils::load_signed_solana_transaction(&args.signed_tx_path)?;
    println!(
        "Loaded combined signed transaction data from: {}",
        args.signed_tx_path.display()
    );

    print_transaction_result(
        config,
        submit_solana_transaction(&config.url, &signed_tx_data),
    )?;

    Ok(())
}
