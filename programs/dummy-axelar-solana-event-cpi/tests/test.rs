use anchor_discriminators::Discriminator;
use dummy_axelar_solana_event_cpi::{
    instruction::emit_event, processor::MemoSentEvent, ID as PROGRAM_ID,
};
use event_cpi_test_utils::assert_event_cpi;
use solana_program::{instruction::AccountMeta, pubkey::Pubkey, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};

#[tokio::test]
async fn test_emit_memo_cpi_event() {
    // Setup the test environment with our program
    let program_test = ProgramTest::new(
        "dummy_axelar_solana_event_cpi",
        PROGRAM_ID,
        processor!(dummy_axelar_solana_event_cpi::processor::process_instruction),
    );

    let (banks_client, payer, recent_blockhash) = program_test.start().await;

    // Test parameters
    let memo = "Hello from integration test!".to_string();
    let test_keypair = Keypair::new();

    // Fund the test account
    let fund_instruction = system_instruction::transfer(
        &payer.pubkey(),
        &test_keypair.pubkey(),
        1_000_000, // 0.001 SOL
    );

    let fund_transaction = Transaction::new_signed_with_payer(
        &[fund_instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    banks_client
        .process_transaction(fund_transaction)
        .await
        .unwrap();

    // Create the emit_event instruction
    let mut instruction = emit_event(&test_keypair.pubkey(), memo.clone()).unwrap();

    // Derive the event authority PDA
    let (event_authority, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &PROGRAM_ID);

    // Add required accounts for event CPI functionality
    instruction
        .accounts
        .push(AccountMeta::new_readonly(event_authority, false));

    instruction
        .accounts
        .push(AccountMeta::new_readonly(PROGRAM_ID, false));

    // Create and process the transaction
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer, &test_keypair],
        recent_blockhash,
    );

    // Use simulate_transaction to get inner instructions
    let simulation_result = banks_client
        .simulate_transaction(transaction.clone())
        .await
        .unwrap();

    // Verify the transaction succeeded
    assert!(simulation_result.result.is_some_and(|r| r.is_ok()));

    // Extract inner instructions
    let inner_ixs = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    assert!(!inner_ixs.is_empty());

    // Find the event CPI instruction
    let expected_event = MemoSentEvent {
        sender: test_keypair.pubkey(),
        memo: memo.clone(),
    };

    assert_event_cpi(&expected_event, &inner_ixs);

    // Also process the transaction to ensure it actually works
    banks_client.process_transaction(transaction).await.unwrap();
}
