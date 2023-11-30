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
async fn teste_call_contract_instruction() {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let sender = Keypair::new();
    let destination_chain = "ethereum";
    let destination_contract_address = "0x2F43DDFf564Fb260dbD783D55fc6E4c70Be18862";
    let payload = rand_array::<100>();

    let instruction = gateway::instruction::call_contract(
        gateway::id(),
        sender.pubkey(),
        destination_chain,
        destination_contract_address,
        &payload,
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
    let _tx_meta = metadata.expect("transaction to have metadata");
    // TODO: check created logs
}
