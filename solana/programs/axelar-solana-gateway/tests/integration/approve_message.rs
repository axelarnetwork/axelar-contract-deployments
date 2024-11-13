use axelar_rkyv_encoding::hasher::merkle_trait::Merkle;
use axelar_rkyv_encoding::hasher::merkle_tree::{SolanaSyscallHasher};
use axelar_rkyv_encoding::types::{Payload, PayloadElement};
use axelar_solana_gateway::events::{ArchivedGatewayEvent, MessageApproved};
use axelar_solana_gateway::instructions::approve_messages;
use axelar_solana_gateway::state::incoming_message::{IncomingMessage, IncomingMessageWrapper};
use axelar_solana_gateway::{get_incoming_message_pda, hasher_impl};
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
    let messages = make_messages(message_count, metadata.domain_separator);
    let payload = Payload::new_messages(messages.clone());
    let payload_merkle_root =
        <Payload as Merkle<SolanaSyscallHasher>>::calculate_merkle_root(&payload).unwrap();
    let verification_session_pda = metadata
        .init_payload_session_and_sign(&metadata.signers.clone(), payload_merkle_root)
        .await
        .unwrap();
    let payload_leaves = <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&payload);
    let proofs = <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&payload);
    let mut counter = 0;
    for (leave, proof) in payload_leaves.zip_eq(proofs) {
        let PayloadElement::Message(message) = leave.element else {
            panic!("invalid message type");
        };
        let payload_hash = message.message.payload_hash;
        let command_id = message.message.cc_id().command_id(hasher_impl());
        let (incoming_message_pda, incoming_message_pda_bump) =
            get_incoming_message_pda(&command_id);

        let expected_event = MessageApproved {
            command_id,
            source_chain: message.message.cc_id().chain().into(),
            message_id: message.message.cc_id().id().into(),
            source_address: message.message.source_address.clone(),
            destination_address: message.message.destination_address.clone(),
            payload_hash,
        };
        let ix = approve_messages(
            message,
            &proof,
            payload_merkle_root,
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
            message: IncomingMessage::new(payload_hash),
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

    let messages_batch_one = make_messages(2, metadata.domain_separator);
    let messages_batch_two = {
        let mut new_messages = make_messages(3, metadata.domain_separator);
        new_messages.extend_from_slice(&messages_batch_one);
        new_messages
    };

    // approve the initial message batch
    metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages_batch_one)
        .await
        .unwrap();

    // approve the second message batch
    let payload = Payload::new_messages(messages_batch_two.to_vec());
    let payload_merkle_root =
        <Payload as Merkle<SolanaSyscallHasher>>::calculate_merkle_root(&payload).unwrap();
    let payload_leaves = <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&payload);
    let proofs = <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&payload);
    let verification_session_pda = metadata
        .init_payload_session_and_sign(&metadata.signers.clone(), payload_merkle_root)
        .await
        .unwrap();
    let mut events_counter = 0;
    let mut message_counter = 0;
    for (message_leaf_node, message_proof) in payload_leaves.zip(proofs) {
        let tx_result = metadata
            .approve_message(
                payload_merkle_root,
                message_leaf_node.clone(),
                message_proof,
                verification_session_pda,
            )
            .await
            .unwrap();
        message_counter += 1;

        let PayloadElement::Message(message) = message_leaf_node.element else {
            panic!("invalid message type");
        };

        if let Some(emitted_event) = get_gateway_events(&tx_result).pop() {
            if let ArchivedGatewayEvent::MessageApproved(_) = emitted_event.parse() {
                events_counter += 1;
            } else {
                panic!("should not end up here");
            }
        };

        // Assert PDA state for message approval
        let payload_hash = message.message.payload_hash;
        let command_id = message.message.cc_id().command_id(hasher_impl());
        let (incoming_message_pda, incoming_message_pda_bump) =
            get_incoming_message_pda(&command_id);

        let account = metadata.incoming_message(incoming_message_pda).await;
        let expected_message = IncomingMessageWrapper {
            message: IncomingMessage::new(payload_hash),
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

    let messages_batch_one = make_messages(2, metadata.domain_separator);

    // approve the batch
    metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages_batch_one)
        .await
        .unwrap();

    // approve the batch many times
    for _ in 0..3 {
        let payload = Payload::new_messages(messages_batch_one.to_vec());
        let payload_merkle_root =
            <Payload as Merkle<SolanaSyscallHasher>>::calculate_merkle_root(&payload).unwrap();
        let payload_leaves = <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&payload);
        let proofs = <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&payload);

        let (verification_session_pda, _bump) =
            axelar_solana_gateway::get_signature_verification_pda(
                &metadata.gateway_root_pda,
                &payload_merkle_root,
            );
        for (message_leaf_node, message_proof) in payload_leaves.zip(proofs) {
            metadata
                .approve_message(
                    payload_merkle_root,
                    message_leaf_node.clone(),
                    message_proof,
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
    let payload = Payload::new_messages(make_messages(2, metadata.domain_separator));
    let payload_merkle_root =
        <Payload as Merkle<SolanaSyscallHasher>>::calculate_merkle_root(&payload).unwrap();

    // Initialize and sign the payload session
    let verification_session_pda = metadata
        .init_payload_session_and_sign(&metadata.signers.clone(), payload_merkle_root)
        .await
        .unwrap();

    // Create a fake message that is not part of the payload
    let fake_payload = Payload::new_messages(make_messages(1, metadata.domain_separator));
    let fake_payload_merkle_root =
        <Payload as Merkle<SolanaSyscallHasher>>::calculate_merkle_root(&fake_payload).unwrap();

    let fake_leaves = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&fake_payload);
    let fake_proofs = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&fake_payload);
    let valid_leaves = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&payload);
    let valid_proofs = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&payload);
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
        for (message_leaf_node, message_proof) in leaves.zip(proofs) {
            dbg!(idx);
            metadata
                .approve_message(
                    merkle_root,
                    message_leaf_node,
                    message_proof,
                    verification_session_pda,
                )
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
    let new_verifier_set_payload_hash = new_verifier_set.verifier_set().payload_hash();
    let payload = Payload::new_verifier_set(new_verifier_set.verifier_set());

    // Initialize and sign the payload session
    let verification_session_pda = metadata
        .init_payload_session_and_sign(&metadata.signers.clone(), new_verifier_set_payload_hash)
        .await
        .unwrap();

    // Create a fake message that is not part of the payload
    let fake_payload = Payload::new_messages(make_messages(1, metadata.domain_separator));
    let fake_payload_merkle_root =
        <Payload as Merkle<SolanaSyscallHasher>>::calculate_merkle_root(&fake_payload).unwrap();

    let fake_leaves = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&fake_payload);
    let fake_proofs = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&fake_payload);
    let _valid_leaves = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_leaves(&payload);
    let valid_proofs = || <Payload as Merkle<SolanaSyscallHasher>>::merkle_proofs(&payload);
    for (idx, (merkle_root, leaves, proofs)) in [
        (fake_payload_merkle_root, fake_leaves(), fake_proofs()),
        (fake_payload_merkle_root, fake_leaves(), valid_proofs()),
        (new_verifier_set_payload_hash, fake_leaves(), fake_proofs()),
        (new_verifier_set_payload_hash, fake_leaves(), valid_proofs()),
        // note: we don't test for `valid leaves` because we cannot derive a command id for it.
    ]
    .into_iter()
    .enumerate()
    {
        for (message_leaf_node, message_proof) in leaves.zip(proofs) {
            dbg!(idx);
            metadata
                .approve_message(
                    merkle_root,
                    message_leaf_node,
                    message_proof,
                    verification_session_pda,
                )
                .await
                .unwrap_err();
        }
    }
}
