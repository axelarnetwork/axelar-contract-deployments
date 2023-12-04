use std::error::Error;

use gateway::events::GatewayEvent;
use random_array::rand_array;
use solana_program_test::{processor, tokio, BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
    )
}

#[tokio::test]
async fn test_queue_message() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let message_id = rand_array::<50>();
    let proof = rand_array::<100>();
    let payload = rand_array::<100>();

    let instruction = gateway::instruction::queue(gateway::id(), &message_id, &proof, &payload)
        .expect("valid instruction construction");

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    let BanksTransactionResultWithMetadata { result, metadata } = banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .expect("transaction to be successful");
    assert!({ result.is_ok() });
    let _tx_meta = metadata.expect("transaction to have metadata");

    // TODO: check created message account
}

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
    )
    .expect("valid instruction construction");

    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } = banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .expect("transaction to be successful");
    assert!({ result.is_ok() });
    let tx_meta = metadata.expect("transaction to have metadata");

    let event = tx_meta
        .log_messages
        .iter()
        .filter_map(|log: &String| GatewayEvent::parse_log(log.as_str()))
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
