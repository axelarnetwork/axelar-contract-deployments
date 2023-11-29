use random_array::rand_array;
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::signature::Signer;
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

    let instruction =
        gateway::instruction::queue(gateway::id(), &message_id, &proof, &payload).unwrap();

    let mut transaction = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // TODO: check created message account
}
