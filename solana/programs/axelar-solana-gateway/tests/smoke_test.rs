use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::sysvar;
use solana_program_test::*;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

#[tokio::test]
async fn smoke_test() {
    let program_id = axelar_solana_gateway::ID;
    let (mut banks_client, payer, recent_blockhash) = ProgramTest::new(
        "axelar_solana_gateway",
        program_id,
        processor!(axelar_solana_gateway::entrypoint::process_instruction),
    )
    .start()
    .await;

    let mut transaction = Transaction::new_with_payer(
        &[Instruction::new_with_bincode(
            program_id,
            &(),
            vec![
                AccountMeta::new(sysvar::clock::id(), false),
                AccountMeta::new(sysvar::rent::id(), false),
            ],
        )],
        Some(&payer.pubkey()),
    );
    transaction.sign(&[&payer], recent_blockhash);
    let result = banks_client.process_transaction(transaction).await;

    let err = result.unwrap_err().to_string();
    assert!(err.contains("Error processing Instruction"));
    assert!(err.contains("Failed to serialize or deserialize account data"));
}
