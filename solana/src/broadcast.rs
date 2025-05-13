use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

use axelar_solana_gateway::num_traits::FromPrimitive;
use eyre::eyre;
use solana_client::{
    client_error::ClientErrorKind, rpc_client::RpcClient, rpc_request::RpcResponseErrorData,
    rpc_response::RpcSimulateTransactionResult,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    instruction::Instruction as SolanaInstruction,
    instruction::InstructionError,
    message::Message,
    pubkey::Pubkey,
    signature::Signature,
    transaction::{Transaction, TransactionError},
};

use crate::config::Config;
use crate::types::SignedSolanaTransaction;
use crate::utils::{self, print_transaction_result};

#[derive(Debug, Clone)]
pub struct BroadcastArgs {
    pub signed_tx_path: PathBuf,
}

fn submit_solana_transaction(
    url: &str,
    signed_tx_data: &SignedSolanaTransaction,
) -> eyre::Result<Signature> {
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
        .collect::<eyre::Result<Vec<_>>>()?;

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
        .collect::<eyre::Result<HashMap<_, _>>>()?;

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
        eyre::bail!(
            "Cannot broadcast: Missing signature for one or more required signers during final reconstruction."
        );
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
        eyre::bail!(
            "Signature count mismatch during reconstruction: Expected {} based on message header, but gathered {}.",
            transaction.signatures.len(),
            ordered_signatures.len()
        );
    }
    transaction.signatures = ordered_signatures;
    transaction.message.recent_blockhash = recent_blockhash;

    println!(
        "Transaction reconstructed with blockhash: {}",
        recent_blockhash
    );

    if let Err(e) = transaction.verify() {
        eyre::bail!(
            "Constructed transaction failed structural verification: {}",
            e
        );
    }

    println!("Connecting to RPC client at {}", url);
    let rpc_client = RpcClient::new_with_commitment(url.to_string(), CommitmentConfig::confirmed());
    let tx_to_send = transaction;

    match rpc_client.simulate_transaction(&tx_to_send) {
        Ok(sim_result) => {
            if let Some(units) = sim_result.value.units_consumed {
                println!("Transaction simulation used {} compute units", units);
                // If we're using significant compute units (>70% of default), we should log this
                if units > 150_000 {
                    println!(
                        "WARNING: Transaction using significant compute units ({}). If this transaction fails with 'exceeded CUs meter', you'll need to add compute budget.",
                        units
                    );
                }
            }
        }
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
            ) = client_err.kind()
            {
                axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
                    .is_some_and(|gw_err| gw_err.should_relayer_proceed())
            } else if let ClientErrorKind::TransactionError(TransactionError::InstructionError(
                _,
                InstructionError::Custom(err_code),
            )) = client_err.kind()
            {
                axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
                    .is_some_and(|gw_err| gw_err.should_relayer_proceed())
            } else {
                false
            };

            if should_continue {
                println!(
                    "Transaction error: GatewayError, but it's a recoverable error - continuing with next transaction"
                );
                Ok(Signature::default())
            } else {
                if let ClientErrorKind::RpcError(
                    solana_client::rpc_request::RpcError::RpcResponseError {
                        data: RpcResponseErrorData::SendTransactionPreflightFailure(sim_result),
                        ..
                    },
                ) = client_err.kind()
                {
                    eprintln!(" -> Preflight Simulation Failure Result: {:?}", sim_result);
                } else if let ClientErrorKind::TransactionError(tx_err) = client_err.kind() {
                    eprintln!(" -> Transaction Error Detail: {:?}", tx_err);
                }

                Err(eyre!("RPC client error: {}", client_err))
            }
        }
    }
}

pub fn broadcast_solana_transaction(args: &BroadcastArgs, config: &Config) -> eyre::Result<()> {
    println!("Starting Solana transaction broadcast...");

    let signed_tx_data = match utils::load_signed_solana_transaction(&args.signed_tx_path) {
        Ok(tx_data) => tx_data,
        Err(e) if e.to_string().contains("json") => {
            eyre::bail!(
                "Failed to parse transaction file. Make sure you're using a signed transaction file (*.signed.json) \
                generated by the 'combine' command, not directly from 'sign' or 'generate'. \
                If you've only signed with one key, run 'combine' first: {}",
                e
            );
        }
        Err(e) => return Err(e),
    };
    println!(
        "Loaded combined signed transaction data from: {}",
        args.signed_tx_path.display()
    );

    match submit_solana_transaction(&config.url, &signed_tx_data) {
        Ok(signature) => {
            if signature == Signature::default() {
                println!(
                    "Transaction had a recoverable error - operation complete with recoverable error"
                );
                Ok(())
            } else {
                print_transaction_result(config, Ok(signature))
            }
        }
        Err(err) => print_transaction_result(config, Err(err)),
    }
}
