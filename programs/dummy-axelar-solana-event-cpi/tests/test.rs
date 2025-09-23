use dummy_axelar_solana_event_cpi::{
    instruction::emit_event, processor::MemoSentEvent, ID as PROGRAM_ID,
};
use event_cpi::Discriminator;
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
    let inner_instructions = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap();

    // Find the event CPI instruction
    let mut found_event = None;

    for inner_ix_group in inner_instructions {
        for inner_ix in inner_ix_group {
            let inner_ix = inner_ix.instruction;

            let data = inner_ix.data;

            // Check if it starts with the event tag
            if data.len() >= 8 && &data[0..8] == event_cpi::EVENT_IX_TAG_LE {
                // Extract the event data (skip the 8-byte tag)
                let event_data = &data[8..];

                // Check if this is a MemoSentEvent (discriminator match)
                if event_data.len() >= 8 && &event_data[0..8] == MemoSentEvent::DISCRIMINATOR {
                    // Deserialize the event
                    if let Ok(event) = borsh::BorshDeserialize::try_from_slice(&event_data[8..]) {
                        found_event = Some(event);
                        break;
                    }
                }
            }
        }
    }

    // Verify we found the event
    let event: MemoSentEvent =
        found_event.expect("Expected to find MemoSentEvent in inner instructions");

    assert_eq!(event.sender, test_keypair.pubkey());
    assert_eq!(event.memo, memo);

    // Also process the transaction to ensure it actually works
    banks_client.process_transaction(transaction).await.unwrap();
}
