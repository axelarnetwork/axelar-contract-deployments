// #![cfg(feature = "test-sbf")]

use std::error::Error;

use random_array::rand_array;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::utils::program_test;

#[tokio::test]
async fn test_queue_message() -> Result<(), Box<dyn Error>> {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let message_id = rand_array::<50>();
    let proof = rand_array::<100>();
    let payload = rand_array::<100>();

    let instruction = gateway::instruction::queue(gateway::id(), &message_id, &proof, &payload)?;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let BanksTransactionResultWithMetadata { result, metadata } = banks_client
        .process_transaction_with_metadata(transaction)
        .await?;

    assert!({ result.is_ok() });
    let _tx_meta = metadata.ok_or("foo")?;

    // TODO: check created message account
    Ok(())
}
