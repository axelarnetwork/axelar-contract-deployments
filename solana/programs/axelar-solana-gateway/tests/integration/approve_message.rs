use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::{MerkleisedMessage, MerkleisedPayload};
use axelar_solana_encoding::types::messages::Messages;
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::LeafHash;
use axelar_solana_gateway::events::{ArchivedGatewayEvent, MessageApproved};
use axelar_solana_gateway::get_incoming_message_pda;
use axelar_solana_gateway::instructions::approve_messages;
use axelar_solana_gateway::state::incoming_message::{
    command_id, IncomingMessage, IncomingMessageWrapper,
};
use axelar_solana_gateway_test_fixtures::gateway::{
    get_gateway_events, make_messages, make_verifier_set,
};
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use itertools::Itertools;
use pretty_assertions::assert_eq;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn successfully_approves_messages() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let message_count = 10;
    let messages = make_messages(message_count);
    let payload = Payload::Messages(Messages(messages.clone()));
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();
    let mut counter = 0;
    let MerkleisedPayload::NewMessages { messages } = execute_data.payload_items else {
        unreachable!()
    };
    for message_info in messages {
        let hash = message_info.leaf.message.hash::<SolanaSyscallHasher>();
        let command_id = command_id(
            &message_info.leaf.message.cc_id.chain,
            &message_info.leaf.message.cc_id.id,
        );
        let (incoming_message_pda, incoming_message_pda_bump) =
            get_incoming_message_pda(&command_id);

        let expected_event = MessageApproved {
            command_id,
            source_chain: message_info.leaf.message.cc_id.chain.clone(),
            message_id: message_info.leaf.message.cc_id.id.clone(),
            source_address: message_info.leaf.message.source_address.clone(),
            destination_address: message_info.leaf.message.destination_address.clone(),
            payload_hash: message_info.leaf.message.payload_hash,
        };
        let ix = approve_messages(
            message_info,
            execute_data.payload_merkle_root,
            metadata.gateway_root_pda,
            metadata.payer.pubkey(),
            verification_session_pda,
            incoming_message_pda,
            incoming_message_pda_bump,
        )
        .unwrap();
        let tx_result = metadata.send_tx(&[ix]).await.unwrap();

        // Assert event
        let emitted_event = get_gateway_events(&tx_result).pop().unwrap();
        let ArchivedGatewayEvent::MessageApproved(emitted_event) = emitted_event.parse() else {
            panic!("unexpected event");
        };
        assert_eq!(*emitted_event, expected_event);

        // Assert PDA state for message approval
        let account = metadata.incoming_message(incoming_message_pda).await;
        let expected_message = IncomingMessageWrapper {
            message: IncomingMessage::new(hash),
            bump: incoming_message_pda_bump,
            _padding_bump: [0; 7],
            _padding_size: [0; 32],
        };
        assert_eq!(account, expected_message);
        counter += 1;
    }
    assert_eq!(counter, message_count);
}

// can approve the same message from many batches
#[tokio::test]
async fn successfully_idempotent_approvals_across_batches() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;

    let messages_batch_one = make_messages(1);
    let messages_batch_two = {
        let mut new_messages = make_messages(1);
        new_messages.extend_from_slice(&messages_batch_one);
        new_messages
    };

    // approve the initial message batch
    let m = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages_batch_one)
        .await
        .unwrap();
    dbg!(&m);

    // approve the second message batch
    let payload = Payload::Messages(Messages(messages_batch_two.clone()));
    let execute_data_batch_two =
        metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data_batch_two)
        .await
        .unwrap();
    let MerkleisedPayload::NewMessages {
        messages: merkle_messages_batch_two,
    } = execute_data_batch_two.payload_items
    else {
        unreachable!()
    };
    let mut events_counter = 0;
    let mut message_counter = 0;
    for message_info in merkle_messages_batch_two {
        dbg!(&message_info);
        let hash = message_info.leaf.message.hash::<SolanaSyscallHasher>();
        let tx_result = metadata
            .approve_message(
                execute_data_batch_two.payload_merkle_root,
                message_info.clone(),
                verification_session_pda,
            )
            .await
            .unwrap();
        message_counter += 1;

        if let Some(emitted_event) = get_gateway_events(&tx_result).pop() {
            if let ArchivedGatewayEvent::MessageApproved(_) = emitted_event.parse() {
                events_counter += 1;
            } else {
                panic!("should not end up here");
            }
        };

        // Assert PDA state for message approval
        let command_id = command_id(
            &message_info.leaf.message.cc_id.chain,
            &message_info.leaf.message.cc_id.id,
        );
        let (incoming_message_pda, incoming_message_pda_bump) =
            get_incoming_message_pda(&command_id);

        let account = metadata.incoming_message(incoming_message_pda).await;
        let expected_message = IncomingMessageWrapper {
            message: IncomingMessage::new(hash),
            bump: incoming_message_pda_bump,
            _padding_bump: [0; 7],
            _padding_size: [0; 32],
        };
        assert_eq!(account, expected_message);
    }

    assert_eq!(
        events_counter,
        messages_batch_two.len() - messages_batch_one.len(),
        "expected new unique events in the second batch"
    );
    assert_eq!(
        message_counter,
        messages_batch_two.len(),
        "expected 4 total messages to be processed in the second batch"
    );
}

// can approve the same message from the same batch many times
#[tokio::test]
async fn successfully_idempotent_approvals_many_times_same_batch() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;

    let messages = make_messages(2);

    // approve the batch
    let payload = Payload::Messages(Messages(messages.clone()));
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    // approve the batch many times
    for _ in 0..3 {
        let MerkleisedPayload::NewMessages { messages } = execute_data.payload_items.clone() else {
            unreachable!()
        };

        for message_info in messages {
            metadata
                .approve_message(
                    execute_data.payload_merkle_root,
                    message_info.clone(),
                    verification_session_pda,
                )
                .await
                .unwrap();
        }
    }
}

// cannot approve a message from a different payload
#[tokio::test]
async fn fails_to_approve_message_not_in_payload() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;

    // Create a payload with a batch of messages
    let payload = Payload::Messages(Messages(make_messages(2)));
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let MerkleisedPayload::NewMessages {
        messages: approved_messages,
    } = execute_data.payload_items.clone()
    else {
        unreachable!();
    };
    let payload_merkle_root = execute_data.payload_merkle_root;

    // Initialize and sign the payload session
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    // Create a fake message that is not part of the payload
    let fake_payload = Payload::Messages(Messages(make_messages(1)));
    let fake_execute_data =
        metadata.construct_execute_data(&metadata.signers.clone(), fake_payload);
    let MerkleisedPayload::NewMessages {
        messages: fake_messages,
    } = fake_execute_data.payload_items
    else {
        unreachable!();
    };
    let fake_payload_merkle_root = fake_execute_data.payload_merkle_root;

    let fm = || fake_messages.clone().into_iter();
    let fake_leaves = || fm().map(|x| x.leaf).collect_vec();
    let fake_proofs = || fm().map(|x| x.proof).collect_vec();
    let ap = || approved_messages.clone().into_iter();
    let valid_leaves = || ap().map(|x| x.leaf).collect_vec();
    let valid_proofs = || ap().map(|x| x.proof).collect_vec();
    for (idx, (merkle_root, leaves, proofs)) in [
        (fake_payload_merkle_root, fake_leaves(), fake_proofs()),
        (fake_payload_merkle_root, fake_leaves(), valid_proofs()),
        (fake_payload_merkle_root, valid_leaves(), valid_proofs()),
        (payload_merkle_root, fake_leaves(), fake_proofs()),
        (payload_merkle_root, fake_leaves(), valid_proofs()),
        (payload_merkle_root, valid_leaves(), fake_proofs()),
    ]
    .into_iter()
    .enumerate()
    {
        for (leaf, proof) in leaves.into_iter().zip(proofs.into_iter()) {
            dbg!(idx);
            let new_message_info = MerkleisedMessage { leaf, proof };
            metadata
                .approve_message(merkle_root, new_message_info, verification_session_pda)
                .await
                .unwrap_err();
        }
    }
}

// cannot approve a message using verifier set payload hash
#[tokio::test]
async fn fails_to_approve_message_using_verifier_set_as_the_root() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;

    // Create a payload with a batch of messages
    let new_verifier_set = make_verifier_set(&[500, 200], 1, metadata.domain_separator);
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);

    // Initialize and sign the payload session
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();
    let MerkleisedPayload::VerifierSetRotation {
        new_verifier_set_merkle_root,
    } = execute_data.payload_items
    else {
        unreachable!();
    };

    // Create a fake message that is not part of the payload
    let fake_payload = Payload::Messages(Messages(make_messages(1)));
    let fake_execute_data =
        metadata.construct_execute_data(&metadata.signers.clone(), fake_payload);
    let MerkleisedPayload::NewMessages {
        messages: fake_messages,
    } = fake_execute_data.payload_items
    else {
        unreachable!();
    };
    let fake_payload_merkle_root = fake_execute_data.payload_merkle_root;

    let fm = || fake_messages.clone().into_iter();
    let fake_leaves = || fm().map(|x| x.leaf).collect_vec();
    let fake_proofs = || fm().map(|x| x.proof).collect_vec();

    // Create a fake message that is not part of the payload
    for (idx, (merkle_root, leaves, proofs)) in [
        (fake_payload_merkle_root, fake_leaves(), fake_proofs()),
        (new_verifier_set_merkle_root, fake_leaves(), fake_proofs()),
    ]
    .into_iter()
    .enumerate()
    {
        for (leaf, proof) in leaves.into_iter().zip(proofs.into_iter()) {
            dbg!(idx);
            let new_message_info = MerkleisedMessage { leaf, proof };
            metadata
                .approve_message(merkle_root, new_message_info, verification_session_pda)
                .await
                .unwrap_err();
        }
    }
}
