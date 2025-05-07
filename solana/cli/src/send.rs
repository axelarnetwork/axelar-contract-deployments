use solana_clap_v3_utils::keypair::signer_from_path;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::transaction::Transaction;

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::SendArgs;
use crate::utils::print_transaction_result;

pub(crate) fn build_and_send_solana_transaction(
    send_args: &SendArgs,
    config: &Config,
    instructions: Vec<Instruction>,
) -> Result<()> {
    let rpc_client = RpcClient::new_with_commitment(&config.url, CommitmentConfig::confirmed());
    let message = Message::new(&instructions, Some(&send_args.fee_payer));
    let mut transaction = Transaction::new_unsigned(message);

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

    let blockhash = rpc_client.get_latest_blockhash()?;
    transaction.sign(&signers, blockhash);

    Ok(print_transaction_result(
        config,
        rpc_client
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| AppError::from(e)),
    )?)
}
