use std::rc::Rc;

use axelar_solana_gateway::num_traits::FromPrimitive;
use eyre::eyre;
use solana_clap_v3_utils::keypair::signer_from_path;
use solana_client::client_error::ClientErrorKind;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_request::RpcResponseErrorData;
use solana_client::rpc_response::RpcSimulateTransactionResult;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::InstructionError;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::{Transaction, TransactionError};

use crate::config::Config;
use crate::types::SerializableSolanaTransaction;
use crate::utils::{
    create_compute_budget_instructions, print_transaction_result, DEFAULT_COMPUTE_UNITS,
    DEFAULT_PRIORITY_FEE,
};

#[derive(Debug)]
pub(crate) struct SendArgs {
    pub(crate) fee_payer: Box<dyn Signer>,
    pub(crate) signers: Vec<String>,
}

fn load_signers(
    signers_paths: &[String],
    transaction: &Transaction,
) -> eyre::Result<Vec<Box<dyn solana_sdk::signer::Signer>>> {
    if signers_paths.len() < transaction.signatures.len() {
        eyre::bail!("Not enough signers provided");
    }

    let mut signers = Vec::with_capacity(transaction.signatures.len());
    let signer_context = clap::ArgMatches::default(); // Dummy context

    for signer in signers_paths.iter() {
        let signer = signer_from_path(&signer_context, signer, "signer", &mut None)
            .map_err(|e| eyre!("Failed to load signer '{signer}': {e}"))?;

        signers.push(signer);
    }

    Ok(signers)
}

fn optimize_transaction(
    transaction: &Transaction,
    signers: &[Box<dyn solana_sdk::signer::Signer>],
    rpc_client: &RpcClient,
    blockhash: &solana_sdk::hash::Hash,
) -> eyre::Result<Transaction> {
    let mut transaction = transaction.clone();

    let has_compute_budget = transaction.message.instructions.iter().any(|ix| {
        let program_id = transaction.message.account_keys[ix.program_id_index as usize];
        program_id == solana_sdk::compute_budget::id()
    });

    if has_compute_budget {
        println!("Transaction already has compute budget instructions, skipping optimization");
        transaction.sign(signers, *blockhash);
        return Ok(transaction);
    }

    println!("Simulating transaction before sending...");
    transaction.sign(signers, *blockhash);

    match rpc_client.simulate_transaction(&transaction) {
        Ok(sim_result) => {
            if let Some(units) = sim_result.value.units_consumed {
                println!("Simulation used {units} compute units");

                if units > 150_000 {
                    println!("Transaction needs significant compute units, adding compute budget");
                    return add_compute_budget_to_transaction(&transaction, signers, blockhash);
                }
            }
        }
        Err(err) => {
            println!("Simulation failed: {err:?}, proceeding with regular transaction");
            transaction.sign(signers, *blockhash);
        }
    }

    Ok(transaction)
}

fn add_compute_budget_to_transaction(
    transaction: &Transaction,
    signers: &[Box<dyn solana_sdk::signer::Signer>],
    blockhash: &solana_sdk::hash::Hash,
) -> eyre::Result<Transaction> {
    let message = &transaction.message;

    let original_instructions: Vec<solana_sdk::instruction::Instruction> = message
        .instructions
        .iter()
        .map(|compiled_ix| solana_sdk::instruction::Instruction {
            program_id: message.account_keys[compiled_ix.program_id_index as usize],
            accounts: compiled_ix
                .accounts
                .iter()
                .map(|account_idx| {
                    let pubkey = message.account_keys[*account_idx as usize];
                    solana_sdk::instruction::AccountMeta {
                        pubkey,
                        is_signer: message.is_signer(*account_idx as usize),
                        is_writable: message.is_maybe_writable(*account_idx as usize, None),
                    }
                })
                .collect(),
            data: compiled_ix.data.clone(),
        })
        .collect();

    let compute_budget_instructions =
        create_compute_budget_instructions(DEFAULT_COMPUTE_UNITS, DEFAULT_PRIORITY_FEE);

    let mut all_instructions = compute_budget_instructions;
    all_instructions.extend(original_instructions);

    let fee_payer = transaction.message.account_keys[0];

    let message = solana_sdk::message::Message::new_with_blockhash(
        &all_instructions,
        Some(&fee_payer),
        blockhash,
    );

    let mut optimized_tx = Transaction::new_unsigned(message);
    optimized_tx.sign(signers, *blockhash);

    println!(
        "Added compute budget: {DEFAULT_COMPUTE_UNITS} units with {DEFAULT_PRIORITY_FEE} micro-lamports priority fee"
    );

    Ok(optimized_tx)
}

fn handle_transaction_error(err: solana_client::client_error::ClientError) -> eyre::Result<bool> {
    let should_continue = if let ClientErrorKind::RpcError(
        solana_client::rpc_request::RpcError::RpcResponseError {
            data:
                RpcResponseErrorData::SendTransactionPreflightFailure(RpcSimulateTransactionResult {
                    err:
                        Some(TransactionError::InstructionError(_, InstructionError::Custom(err_code))),
                    ..
                }),
            ..
        },
    ) = err.kind()
    {
        axelar_solana_gateway::error::GatewayError::from_u32(*err_code)
            .is_some_and(|gw_err| gw_err.should_relayer_proceed())
    } else if let ClientErrorKind::TransactionError(TransactionError::InstructionError(
        _,
        InstructionError::Custom(err_code),
    )) = err.kind()
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
        Ok(true)
    } else {
        eyre::bail!("Transaction simulation error: {err:?}");
    }
}

pub(crate) fn sign_and_send_transactions(
    send_args: SendArgs,
    config: &Config,
    serializable_txs: Vec<SerializableSolanaTransaction>,
) -> eyre::Result<()> {
    let rpc_client = RpcClient::new_with_commitment(&config.url, CommitmentConfig::confirmed());
    let mut results = Vec::new();

    let SendArgs { fee_payer, signers } = send_args;
    let shared_payer: Rc<dyn Signer> = Rc::from(fee_payer);

    for serializable_tx in serializable_txs {
        let transaction = serializable_tx.transaction;
        let mut signers = load_signers(&signers, &transaction)?;
        signers.push(Box::new(shared_payer.clone()));

        let blockhash = rpc_client.get_latest_blockhash()?;
        let optimized_tx = optimize_transaction(&transaction, &signers, &rpc_client, &blockhash)?;

        match rpc_client.send_and_confirm_transaction(&optimized_tx) {
            Ok(signature) => {
                results.push(signature);
            }
            Err(err) => {
                eprintln!("Error during transaction: {err}");
                if !handle_transaction_error(err)? {
                    return Err(eyre!("Transaction error"));
                }
            }
        }
    }

    for (i, signature) in results.iter().enumerate() {
        println!("Transaction {}: {}", i + 1, signature);
        print_transaction_result(config, Ok(*signature))?;
    }

    Ok(())
}
