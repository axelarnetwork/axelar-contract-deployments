use axelar_executable::axelar_message_primitives::EncodingScheme;
use axelar_solana_memo_program::instruction::from_axelar_to_solana::build_memo;
use gateway::commands::OwnedCommand;
use gateway::state::GatewayApprovedCommand;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use test_fixtures::axelar_message::custom_message;

use crate::program_test;

#[rstest::rstest]
#[case(EncodingScheme::Borsh)]
#[case(EncodingScheme::AbiEncoding)]
#[tokio::test]
async fn test_successful_validate_message(#[case] encoding_scheme: EncodingScheme) {
    // Setup
    let mut solana_chain = program_test().await;
    let (counter_pda, counter_bump) =
        axelar_solana_memo_program::get_counter_pda(&solana_chain.gateway_root_pda);
    solana_chain
        .fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    // Test scoped constants
    let random_account_used_by_ix = Keypair::new();
    let destination_program_id = axelar_solana_memo_program::id();
    let memo_string = "🐪🐪🐪🐪";

    // Create 2 messages: one we're going to execute and one we're not
    let message_payload = build_memo(
        memo_string.as_bytes(),
        &counter_pda,
        &[&random_account_used_by_ix.pubkey()],
        encoding_scheme,
    );
    let message_to_execute = custom_message(destination_program_id, &message_payload);
    let other_message_in_the_batch = custom_message(destination_program_id, &message_payload);

    // Confidence check: `message_to_execute` and `message_payload` have the same
    // hash.
    assert_eq!(
        *message_to_execute.payload_hash(),
        *(message_payload.hash().unwrap().0)
    );

    let messages = vec![message_to_execute.clone(), other_message_in_the_batch];
    // Action: "Relayer" calls Gateway to approve messages
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            messages.clone(),
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let approve_message_command = OwnedCommand::ApproveMessage(message_to_execute);
    // Action: set message status as executed by calling the destination program
    let tx = solana_chain
        .fixture
        .call_execute_on_axelar_executable(
            &approve_message_command,
            &message_payload,
            &gateway_approved_command_pdas[0],
            &solana_chain.gateway_root_pda,
        )
        .await;

    assert!(tx.result.is_ok(), "transaction failed");
    // Assert
    // First message should be executed
    let gateway_approved_message = solana_chain
        .fixture
        .get_account::<GatewayApprovedCommand>(&gateway_approved_command_pdas[0], &gateway::id())
        .await;
    assert!(gateway_approved_message.is_command_executed());

    // The second message is still in Approved status
    let gateway_approved_message = solana_chain
        .fixture
        .get_account::<GatewayApprovedCommand>(&gateway_approved_command_pdas[1], &gateway::id())
        .await;
    assert!(gateway_approved_message.is_command_approved());

    // We can get the memo from the logs
    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains(&format!(
            "{:?}-{}-{}",
            random_account_used_by_ix.pubkey(),
            false,
            false
        ))),
        "expected memo not found in logs"
    );

    // The counter should have been incremented
    let counter_account = solana_chain
        .fixture
        .get_account::<axelar_solana_memo_program::state::Counter>(
            &counter_pda,
            &axelar_solana_memo_program::id(),
        )
        .await;
    assert_eq!(counter_account.counter, 1);
}
