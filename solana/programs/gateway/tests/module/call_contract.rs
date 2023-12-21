// #![cfg(feature = "test-sbf")]

use std::error::Error;

use gateway::events::GatewayEvent;
use random_array::rand_array;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use crate::utils::program_test;

#[tokio::test]
async fn test_call_contract_instruction() -> Result<(), Box<dyn Error>> {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let sender = Keypair::new();
    let destination_chain = "ethereum";
    let destination_address = hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862")?;
    let payload = rand_array::<32>().to_vec();
    let payload_hash = rand_array::<32>();

    let instruction = gateway::instruction::call_contract(
        gateway::id(),
        sender.pubkey(),
        destination_chain,
        &destination_address,
        &payload,
        payload_hash,
    )?;

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

    let event = metadata
        .ok_or("expected transaction to have metadata")?
        .log_messages
        .iter()
        .filter_map(GatewayEvent::parse_log)
        .next();

    assert_eq!(
        event,
        Some(GatewayEvent::CallContract {
            sender: sender.pubkey(),
            destination_chain: destination_chain.as_bytes().to_vec(),
            destination_address,
            payload,
            payload_hash
        })
    );

    Ok(())
}
