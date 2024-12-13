use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::MerkleisedPayload;
use axelar_solana_encoding::types::messages::{Message, Messages};
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::LeafHash;
use axelar_solana_gateway::instructions::approve_messages;
use axelar_solana_gateway::processor::GatewayEvent;
use axelar_solana_gateway::state::incoming_message::{command_id, IncomingMessage, MessageStatus};
use axelar_solana_gateway::state::message_payload::MessagePayload;
use axelar_solana_gateway::{
    find_message_payload_pda, get_incoming_message_pda, get_validate_message_signing_pda,
};
use axelar_solana_gateway_test_fixtures::gateway::{
    get_gateway_events, random_message, ProgramInvocationState,
};
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use pretty_assertions::assert_eq;
use solana_program_test::tokio;
use solana_sdk::account::Account;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use std::str::FromStr;

/// Helper fn to produce a command id from a message.
pub fn message_to_command_id(message: &Message) -> [u8; 32] {
    command_id(&message.cc_id.chain, &message.cc_id.id)
}

pub async fn get_message_account(
    runner: &mut SolanaAxelarIntegrationMetadata,
    message: &Message,
) -> Option<Account> {
    let command_id = message_to_command_id(message);
    let (message_payload_pda, _bump) = axelar_solana_gateway::find_message_payload_pda(
        runner.gateway_root_pda,
        command_id,
        runner.payer.pubkey(),
    );
    runner
        .try_get_account(&message_payload_pda, &axelar_solana_gateway::ID)
        .await
        .expect("error while getting account")
}

/// Helper fn to approve a single message
pub async fn approve_message(runner: &mut SolanaAxelarIntegrationMetadata, message: Message) {
    let payload = Payload::Messages(Messages(vec![message.clone()]));
    let execute_data = runner.construct_execute_data(&runner.signers.clone(), payload);
    let verification_session_pda = runner
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    let command_id = message_to_command_id(&message);
    let (incoming_message_pda, incoming_message_pda_bump) = get_incoming_message_pda(&command_id);
    let message_info = {
        let MerkleisedPayload::NewMessages { messages } = execute_data.payload_items else {
            unreachable!()
        };
        messages.into_iter().next().unwrap()
    };

    let ix = approve_messages(
        message_info,
        execute_data.payload_merkle_root,
        runner.gateway_root_pda,
        runner.payer.pubkey(),
        verification_session_pda,
        incoming_message_pda,
    )
    .unwrap();
    let tx = runner.send_tx(&[ix]).await.unwrap();

    // Assert event
    let expected_event = axelar_solana_gateway::processor::MessageEvent {
        command_id,
        cc_id_chain: message.cc_id.chain.clone(),
        cc_id_id: message.cc_id.id.clone(),
        source_address: message.source_address.clone(),
        destination_address: Pubkey::from_str(&message.destination_address).unwrap(),
        payload_hash: message.payload_hash,
        destination_chain: message.destination_chain.clone(),
    };
    let emitted_events = get_gateway_events(&tx).pop().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GatewayEvent::MessageApproved(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    assert_eq!(emitted_event, &expected_event);

    let (_, signing_pda_bump) =
        get_validate_message_signing_pda(expected_event.destination_address, command_id);

    // Assert PDA state for message approval
    let account = runner.incoming_message(incoming_message_pda).await;
    let expected_message = IncomingMessage::new(
        incoming_message_pda_bump,
        signing_pda_bump,
        MessageStatus::Approved,
        message.hash::<SolanaSyscallHasher>(),
        message.payload_hash,
    );
    assert_eq!(account, expected_message);
}

/// Helper fn to initialize a single message payload account
pub async fn initialize_message_payload_pda(
    runner: &mut SolanaAxelarIntegrationMetadata,
    message: &Message,
    buffer_size: u64,
) {
    approve_message(runner, message.clone()).await;

    // Build instruction and send it in a transaction
    let command_id = message_to_command_id(message);

    let ix = axelar_solana_gateway::instructions::initialize_message_payload(
        runner.gateway_root_pda,
        runner.payer.pubkey(),
        command_id,
        buffer_size,
    )
    .unwrap();
    let tx = runner.send_tx(&[ix]).await.unwrap();
    assert!(tx.result.is_ok());

    // Assert that the new MessagePayload PDA has the correct size and expected data
    let mut message_payload_account = get_message_account(runner, message)
        .await
        .expect("error getting account");

    let message_payload =
        MessagePayload::from_borrowed_account_data(&mut message_payload_account.data)
            .expect("valid message payload account contents");

    assert_eq!(message_payload.raw_payload.len(), buffer_size as usize,);
    assert!(message_payload.raw_payload.iter().all(|&x| x == 0));
    assert!(message_payload.payload_hash.iter().all(|&x| x == 0));

    // Check the bump too
    let (_, bump) =
        find_message_payload_pda(runner.gateway_root_pda, command_id, runner.payer.pubkey());
    assert_eq!(*message_payload.bump, bump);
}

#[tokio::test]
async fn successfully_initialize_message_payload_pda() {
    // Setup
    let mut runner = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let message = random_message();
    let buffer_size = 50; // doesn't matter for this test
    initialize_message_payload_pda(&mut runner, &message, buffer_size).await;
}
