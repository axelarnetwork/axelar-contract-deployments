use axelar_executable::EncodingScheme;
use axelar_solana_gateway::get_incoming_message_pda;
use axelar_solana_gateway::processor::MessageEvent;
use axelar_solana_gateway::state::incoming_message::{command_id, MessageStatus};
use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use axelar_solana_memo_program::instruction::from_axelar_to_solana::build_memo;
use borsh::BorshDeserialize;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};

use crate::program_test;

#[rstest::rstest]
#[case(EncodingScheme::Borsh)]
#[case(EncodingScheme::AbiEncoding)]
#[tokio::test]
async fn test_successful_validate_message(#[case] encoding_scheme: EncodingScheme) {
    use std::str::FromStr;

    use axelar_solana_gateway::processor::GatewayEvent;
    // Setup
    use axelar_solana_gateway_test_fixtures::gateway::{
        get_gateway_events, ProgramInvocationState,
    };
    use axelar_solana_memo_program::state::Counter;
    use solana_sdk::pubkey::Pubkey;

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
        .await
        .unwrap();

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
    let mut message_to_execute = random_message();
    message_to_execute.destination_address = destination_program_id.to_string();
    message_to_execute.payload_hash = *message_payload.hash().unwrap().0;

    let mut other_message_in_the_batch = random_message();
    other_message_in_the_batch.destination_address = destination_program_id.to_string();
    other_message_in_the_batch.payload_hash = *message_payload.hash().unwrap().0;

    let messages = vec![
        message_to_execute.clone(),
        other_message_in_the_batch.clone(),
    ];
    // Action: "Relayer" calls Gateway to approve messages
    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &messages)
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) = get_incoming_message_pda(&command_id(
        &message_to_execute.cc_id.chain,
        &message_to_execute.cc_id.id,
    ));
    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message_to_execute.cc_id)
        .unwrap()
        .clone();
    let tx = solana_chain
        .execute_on_axelar_executable(
            merkelised_message.leaf.message.clone(),
            &message_payload.encode().unwrap(),
        )
        .await
        .unwrap();

    // Assert
    // First message should be executed
    let gateway_approved_message = solana_chain.incoming_message(incoming_message_pda).await;
    assert_eq!(
        gateway_approved_message.message.status,
        MessageStatus::Executed
    );

    // The second message is still in Approved status
    let (incoming_message_pda, ..) = get_incoming_message_pda(&command_id(
        &other_message_in_the_batch.cc_id.chain,
        &other_message_in_the_batch.cc_id.id,
    ));
    let gateway_approved_message = solana_chain.incoming_message(incoming_message_pda).await;
    assert_eq!(
        gateway_approved_message.message.status,
        MessageStatus::Approved
    );

    // We can get the memo from the logs
    assert!(
        tx.find_log("🐪🐪🐪🐪").is_some(),
        "expected memo not found in logs"
    );
    assert!(
        tx.find_log(&format!(
            "{:?}-{}-{}",
            random_account_used_by_ix.pubkey(),
            false,
            false
        ))
        .is_some(),
        "expected memo not found in logs"
    );

    // The counter should have been incremented
    let counter_account = solana_chain
        .fixture
        .get_account(&counter_pda, &axelar_solana_memo_program::id())
        .await;
    let counter = Counter::try_from_slice(&counter_account.data).unwrap();
    assert_eq!(counter.counter, 1);

    // Event was logged
    let emitted_events = get_gateway_events(&tx).pop().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GatewayEvent::MessageExecuted(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    let command_id = command_id(
        &merkelised_message.leaf.message.cc_id.chain,
        &merkelised_message.leaf.message.cc_id.id,
    );
    let expected_event = MessageEvent {
        command_id,
        cc_id_chain: merkelised_message.leaf.message.cc_id.chain,
        cc_id_id: merkelised_message.leaf.message.cc_id.id,
        source_address: merkelised_message.leaf.message.source_address,
        destination_address: Pubkey::from_str(&merkelised_message.leaf.message.destination_address)
            .unwrap(),
        payload_hash: merkelised_message.leaf.message.payload_hash,
        destination_chain: merkelised_message.leaf.message.destination_chain,
    };

    assert_eq!(emitted_event, &expected_event);
}
