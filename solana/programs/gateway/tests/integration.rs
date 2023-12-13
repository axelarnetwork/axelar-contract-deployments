// #![cfg(feature = "test-sbf")]

use std::error::Error;

use gateway::events::GatewayEvent;
use random_array::rand_array;
use solana_program::pubkey::Pubkey;
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

#[tokio::test]
async fn test_initialize() -> Result<(), Box<dyn Error>> {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let (root_pda, _bump) = Pubkey::find_program_address(&[&[]], &gateway::id());

    let payload = b"All you need is potatoes!";

    let ix = gateway::instruction::build_initialize_ix(payer.pubkey(), root_pda, payload)?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata {
        result,
        metadata: _,
    } = banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let actual_data = banks_client
        .get_account(root_pda)
        .await
        .expect("get_account")
        .unwrap();

    // [2..] as the data serialized to account consist of 2-byte len descriptor.
    assert_eq!(actual_data.owner, gateway::id());
    assert_eq!(actual_data.data[2..].len(), payload.len());
    assert_eq!(&actual_data.data[2..], payload);

    Ok(())
}
