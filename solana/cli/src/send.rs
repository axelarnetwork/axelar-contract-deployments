use axelar_solana_gateway::num_traits::FromPrimitive;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_client::{
    client_error::ClientErrorKind, rpc_client::RpcClient, rpc_request::RpcResponseErrorData,
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::InstructionError,
    transaction::{Transaction, TransactionError},
};

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{SendArgs, SerializableSolanaTransaction};
use crate::utils::{
    DEFAULT_COMPUTE_UNITS, DEFAULT_PRIORITY_FEE, create_compute_budget_instructions,
    print_transaction_result,
};

pub(crate) fn sign_and_send_transactions(
    send_args: &SendArgs,
    config: &Config,
    serializable_txs: Vec<SerializableSolanaTransaction>,
) -> Result<()> {
    let rpc_client = RpcClient::new_with_commitment(&config.url, CommitmentConfig::confirmed());
    let mut results = Vec::new();

    for serializable_tx in serializable_txs {
        let mut transaction = serializable_tx.transaction;

        if send_args.signers.len() < transaction.signatures.len() {
            return Err(AppError::SigningError(
                "Not enough signers provided".to_string(),
            ));
        }

        let mut signers = Vec::with_capacity(transaction.signatures.len());
        let signer_context = clap::ArgMatches::default(); // Dummy context

        for signer in send_args.signers.iter() {
            let signer =
                signer_from_path(&signer_context, signer, "signer", &mut None).map_err(|e| {
                    AppError::SigningError(format!("Failed to load signer '{}': {}", signer, e))
                })?;

            signers.push(signer);
        }

        // Get the latest blockhash
        let blockhash = rpc_client.get_latest_blockhash()?;

        // Check if the transaction already has compute budget instructions
        let has_compute_budget = transaction.message.instructions.iter().any(|ix| {
            let program_id = transaction.message.account_keys[ix.program_id_index as usize];
            program_id == solana_sdk::compute_budget::id()
        });

        // First, we'll make a transaction with just the original instructions
        transaction.sign(&signers, blockhash);

        // Only try to optimize if there are no compute budget instructions already
        if !has_compute_budget {
            // Try to simulate the transaction to see if it might exceed compute limits
            println!("Simulating transaction before sending...");
            match rpc_client.simulate_transaction(&transaction) {
                Ok(sim_result) => {
                    if let Some(units) = sim_result.value.units_consumed {
                        println!("Simulation used {} compute units", units);

                        // If we're using a significant portion of the compute limit, add a compute budget
                        if units > 150_000 {
                            // Create a new transaction with compute budget instructions
                            println!(
                                "Transaction needs significant compute units, adding compute budget"
                            );

                            // Extract original instructions from the transaction message
                            // Use the transaction's message instructions directly
                            let message = &transaction.message;

                            // Convert CompiledInstructions to regular Instructions
                            let original_instructions: Vec<solana_sdk::instruction::Instruction> =
                                message
                                    .instructions
                                    .iter()
                                    .map(|compiled_ix| solana_sdk::instruction::Instruction {
                                        program_id: message.account_keys
                                            [compiled_ix.program_id_index as usize],
                                        accounts: compiled_ix
                                            .accounts
                                            .iter()
                                            .map(|account_idx| {
                                                let pubkey =
                                                    message.account_keys[*account_idx as usize];
                                                solana_sdk::instruction::AccountMeta {
                                                    pubkey,
                                                    is_signer: message
                                                        .is_signer(*account_idx as usize),
                                                    is_writable: message.is_maybe_writable(
                                                        *account_idx as usize,
                                                        None,
                                                    ),
                                                }
                                            })
                                            .collect(),
                                        data: compiled_ix.data.clone(),
                                    })
                                    .collect();

                            // Create compute budget instructions
                            let compute_budget_instructions = create_compute_budget_instructions(
                                DEFAULT_COMPUTE_UNITS,
                                DEFAULT_PRIORITY_FEE,
                            );

                            // Combine compute budget instructions with original instructions
                            let mut all_instructions = compute_budget_instructions;
                            all_instructions.extend(original_instructions);

                            // Get the fee payer
                            let fee_payer = transaction.message.account_keys[0];

                            // Create a new message with all instructions
                            let message = solana_sdk::message::Message::new_with_blockhash(
                                &all_instructions,
                                Some(&fee_payer),
                                &blockhash,
                            );

                            // Create and sign a new transaction
                            let mut optimized_tx = Transaction::new_unsigned(message);
                            optimized_tx.sign(&signers, blockhash);

                            // Use the optimized transaction instead
                            transaction = optimized_tx;
                            println!(
                                "Added compute budget: {} units with {} micro-lamports priority fee",
                                DEFAULT_COMPUTE_UNITS, DEFAULT_PRIORITY_FEE
                            );
                        }
                    }
                }
                Err(err) => {
                    println!(
                        "Simulation failed: {:?}, proceeding with regular transaction",
                        err
                    );
                    // If simulation fails, just use the original transaction
                    transaction.sign(&signers, blockhash);
                }
            };
        } else {
            println!("Transaction already has compute budget instructions, skipping optimization");
        }

        // Now send the transaction (either original or optimized)
        match rpc_client.send_and_confirm_transaction(&transaction) {
            Ok(signature) => {
                results.push(signature);
            }
            Err(err) => {
                let should_continue = if let ClientErrorKind::RpcError(
                    solana_client::rpc_request::RpcError::RpcResponseError {
                        data:
                            RpcResponseErrorData::SendTransactionPreflightFailure(
                                RpcSimulateTransactionResult {
                                    err:
                                        Some(TransactionError::InstructionError(
                                            _,
                                            InstructionError::Custom(err_code),
                                        )),
                                    ..
                                },
                            ),
                        ..
                    },
                ) = err.kind()
                {
                    axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
                        .is_some_and(|gw_err| gw_err.should_relayer_proceed())
                } else if let ClientErrorKind::TransactionError(
                    TransactionError::InstructionError(_, InstructionError::Custom(err_code)),
                ) = err.kind()
                {
                    axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
                        .is_some_and(|gw_err| gw_err.should_relayer_proceed())
                } else {
                    false
                };

                if should_continue {
                    println!(
                        "Transaction error: GatewayError (code: {:?}), but continuing with next transaction as it's recoverable",
                        err.kind()
                    );
                    continue;
                } else {
                    println!("Transaction error: {:?}", err);
                    return Err(AppError::from(err));
                }
            }
        }
    }

    // Print results
    for (i, signature) in results.iter().enumerate() {
        println!("Transaction {}: {}", i + 1, signature);
        print_transaction_result(config, Ok(*signature))?;
    }

    Ok(())
}
