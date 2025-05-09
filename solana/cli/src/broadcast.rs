use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{BroadcastArgs, BroadcastMultipleArgs, SignedSolanaTransaction};
use crate::utils::{self, print_transaction_result, create_compute_budget_instructions, DEFAULT_COMPUTE_UNITS, DEFAULT_PRIORITY_FEE};
use solana_client::{
    client_error::ClientErrorKind,
    rpc_client::RpcClient,
    rpc_request::RpcResponseErrorData,
    rpc_response::RpcSimulateTransactionResult
};
use solana_sdk::{
    commitment_config::CommitmentConfig, compute_budget::ComputeBudgetInstruction, hash::Hash, 
    instruction::Instruction as SolanaInstruction, instruction::InstructionError, 
    message::Message, pubkey::Pubkey, signature::Signature, transaction::{Transaction, TransactionError}
};
use axelar_solana_gateway::num_traits::FromPrimitive;
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
    
    // Don't automatically add compute budget instructions to avoid duplicates
    // We'll use the original transaction directly
    let tx_to_send = transaction;

    // Simulate the transaction before sending to check if we need to add compute units
    match rpc_client.simulate_transaction(&tx_to_send) {
        Ok(sim_result) => {
            if let Some(units) = sim_result.value.units_consumed {
                println!("Transaction simulation used {} compute units", units);
                // If we're using significant compute units (>70% of default), we should log this
                if units > 150_000 {
                    println!("WARNING: Transaction using significant compute units ({}). If this transaction fails with 'exceeded CUs meter', you'll need to add compute budget.", units);
                }
            }
        },
        Err(err) => {
            println!("Simulation warning: {:?}", err);
        }
    };

    println!("Broadcasting transaction...");
    match rpc_client.send_and_confirm_transaction_with_spinner(&tx_to_send) {
        Ok(tx_signature) => {
            println!("Transaction broadcast and confirmed!");
            Ok(tx_signature)
        }
        Err(client_err) => {
            eprintln!("Error during RPC broadcast/confirmation: {}", client_err);

            // Check if the error is a GatewayError and should proceed
            let should_continue = if let ClientErrorKind::RpcError(
                solana_client::rpc_request::RpcError::RpcResponseError {
                    data: RpcResponseErrorData::SendTransactionPreflightFailure(
                        RpcSimulateTransactionResult {
                            err: Some(TransactionError::InstructionError(_,
                                    InstructionError::Custom(err_code))),
                            ..
                        }
                    ),
                    ..
                }
            ) = client_err.kind() {
                axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
                    .is_some_and(|gw_err| gw_err.should_relayer_proceed())
            } else if let ClientErrorKind::TransactionError(
                TransactionError::InstructionError(_,
                    InstructionError::Custom(err_code))
            ) = client_err.kind() {
                axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
                    .is_some_and(|gw_err| gw_err.should_relayer_proceed())
            } else {
                false
            };

            if should_continue {
                println!("Transaction error: GatewayError, but it's a recoverable error - continuing with next transaction");
                // Return a dummy signature to indicate success for recoverable errors
                Ok(Signature::default())
            } else {
                // Print detailed error information
                if let ClientErrorKind::RpcError(
                    solana_client::rpc_request::RpcError::RpcResponseError {
                        data: RpcResponseErrorData::SendTransactionPreflightFailure(sim_result),
                        ..
                    }
                ) = client_err.kind() {
                    eprintln!(" -> Preflight Simulation Failure Result: {:?}", sim_result);
                } else if let ClientErrorKind::TransactionError(tx_err) = client_err.kind() {
                    eprintln!(" -> Transaction Error Detail: {:?}", tx_err);
                }

                // Return the error as non-recoverable
                Err(AppError::from(client_err))
            }
        }
    }
}

pub fn broadcast_solana_transaction(args: &BroadcastArgs, config: &Config) -> Result<()> {
    println!("Starting Solana transaction broadcast...");

    // Load the signed transaction data, handling potential format issues
    let signed_tx_data = match utils::load_signed_solana_transaction(&args.signed_tx_path) {
        Ok(tx_data) => tx_data,
        Err(AppError::JsonError(e)) => {
            // Provide more helpful error message for JSON parsing errors
            return Err(AppError::BroadcastError(format!(
                "Failed to parse transaction file. Make sure you're using a file generated by the 'combine' command, \
                not directly from 'sign' or 'generate'. If you've only signed with one key, run 'combine' first: {}",
                e
            )));
        },
        Err(e) => return Err(e),
    };
    println!(
        "Loaded combined signed transaction data from: {}",
        args.signed_tx_path.display()
    );

    match submit_solana_transaction(&config.url, &signed_tx_data) {
        Ok(signature) => {
            // Handle the special case where we return a default signature for recoverable errors
            if signature == Signature::default() {
                println!("Transaction had a recoverable error - operation complete with recoverable error");
                Ok(())
            } else {
                print_transaction_result(config, Ok(signature))
            }
        },
        Err(err) => print_transaction_result(config, Err(err)),
    }
}

pub fn broadcast_multiple_transactions(args: &BroadcastMultipleArgs, config: &Config) -> Result<()> {
    println!("Starting Solana batch transaction broadcast...");
    let mut success_count = 0;
    let total_count = args.signed_tx_paths.len();

    for (i, signed_tx_path) in args.signed_tx_paths.iter().enumerate() {
        println!("\nBroadcasting transaction {} of {}...", i + 1, total_count);

        // Load the signed transaction data, handling potential format issues
        let signed_tx_data = match utils::load_signed_solana_transaction(signed_tx_path) {
            Ok(tx_data) => tx_data,
            Err(AppError::JsonError(e)) => {
                // Provide more helpful error message for JSON parsing errors
                return Err(AppError::BroadcastError(format!(
                    "Failed to parse transaction file {}. Make sure you're using files generated by the 'combine' command, \
                    not directly from 'sign' or 'generate'. If you've only signed with one key, run 'combine' first: {}",
                    signed_tx_path.display(), e
                )));
            },
            Err(e) => return Err(e),
        };
        println!(
            "Loaded combined signed transaction data from: {}",
            signed_tx_path.display()
        );

        match submit_solana_transaction(&config.url, &signed_tx_data) {
            Ok(signature) => {
                // Handle the special case where we return a default signature for recoverable errors
                if signature == Signature::default() {
                    println!("Transaction had a recoverable error - continuing with next transaction");
                } else {
                    print_transaction_result(config, Ok(signature))?;
                    success_count += 1;
                }
            },
            Err(err) => {
                // We already checked for GatewayError in submit_solana_transaction,
                // so this is a truly unrecoverable error
                print_transaction_result(config, Err(err))?;
                return Err(AppError::BroadcastError(format!(
                    "Failed at transaction {} of {}. {} succeeded.",
                    i + 1,
                    total_count,
                    success_count
                )));
            }
        }
    }

    println!("\n{} of {} transactions broadcast successfully!", success_count, total_count);
    
    Ok(())
}