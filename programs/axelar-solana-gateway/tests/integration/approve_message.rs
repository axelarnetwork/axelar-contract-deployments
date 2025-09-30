use core::str::FromStr;
use std::iter;

use axelar_solana_encoding::hasher::{NativeHasher, SolanaSyscallHasher};
use axelar_solana_encoding::types::execute_data::{MerkleisedMessage, MerkleisedPayload};
use axelar_solana_encoding::types::messages::Messages;
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::types::verifier_set::verifier_set_hash;
use axelar_solana_encoding::LeafHash;
use axelar_solana_gateway::error::GatewayError;
use axelar_solana_gateway::events::MessageApprovedEvent;
use axelar_solana_gateway::state::incoming_message::{command_id, IncomingMessage, MessageStatus};
use axelar_solana_gateway::{get_incoming_message_pda, get_validate_message_signing_pda};
use axelar_solana_gateway_test_fixtures::gateway::{
    get_gateway_events, make_messages, make_verifier_set, random_message, GetGatewayError,
};
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use event_cpi_test_utils::{assert_event_cpi, find_event_cpi};
use itertools::Itertools;
use pretty_assertions::assert_eq;
use rand::Rng;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;

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

        let message = message_info.leaf.clone().message;
        // First simulate to check events
        let simulation_result = metadata
            .simulate_approve_message(
                execute_data.payload_merkle_root,
                message_info.clone(),
                verification_session_pda,
            )
            .await
            .unwrap();

        // Assert event emitted
        let inner_ixs = simulation_result
            .simulation_details
            .unwrap()
            .inner_instructions
            .unwrap()
            .first()
            .cloned()
            .unwrap();
        assert!(!inner_ixs.is_empty());

        let expected_event = MessageApprovedEvent {
            command_id,
            source_chain: message.cc_id.chain.clone(),
            cc_id: message.cc_id.id.clone(),
            source_address: message.source_address.clone(),
            destination_address: Pubkey::from_str(&message.destination_address).unwrap(),
            payload_hash: message.payload_hash,
            destination_chain: message.destination_chain,
        };

        assert_event_cpi(&expected_event, &inner_ixs);

        // Execute the transaction
        let _tx = metadata
            .approve_message(
                execute_data.payload_merkle_root,
                message_info,
                verification_session_pda,
            )
            .await
            .unwrap();

        let (_, signing_pda_bump) =
            get_validate_message_signing_pda(expected_event.destination_address, command_id);

        // Assert PDA state for message approval
        let account = metadata.incoming_message(incoming_message_pda).await;
        let expected_message = IncomingMessage::new(
            incoming_message_pda_bump,
            signing_pda_bump,
            MessageStatus::approved(),
            hash,
            message.payload_hash,
        );

        assert_eq!(account, expected_message);
        counter += 1;
    }
    assert_eq!(counter, message_count);
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn fail_individual_approval_if_done_many_times() {
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
    let _m = metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &messages_batch_one)
        .await
        .unwrap();

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
        let hash = message_info.leaf.message.hash::<SolanaSyscallHasher>();
        // First simulate to check events
        let simulation_result = metadata
            .simulate_approve_message(
                execute_data_batch_two.payload_merkle_root,
                message_info.clone(),
                verification_session_pda,
            )
            .await;

        // Check the event was emitted in simulation
        if let Some(inner_ixs) = simulation_result
            .ok()
            .and_then(|sim| sim.simulation_details)
            .and_then(|details| details.inner_instructions)
            .and_then(|instructions| instructions.first().cloned())
        {
            let destination_address =
                Pubkey::from_str(&message_info.leaf.message.destination_address).unwrap();
            let command_id = command_id(
                &message_info.leaf.message.cc_id.chain,
                &message_info.leaf.message.cc_id.id,
            );

            let expected_event = MessageApprovedEvent {
                command_id,
                source_chain: message_info.leaf.message.cc_id.chain.clone(),
                cc_id: message_info.leaf.message.cc_id.id.clone(),
                source_address: message_info.leaf.message.source_address.clone(),
                destination_address,
                payload_hash: message_info.leaf.message.payload_hash,
                destination_chain: message_info.leaf.message.destination_chain.clone(),
            };

            if find_event_cpi(&expected_event, &inner_ixs) {
                events_counter += 1;
            }
        }

        // Now execute the transaction
        let tx = metadata
            .approve_message(
                execute_data_batch_two.payload_merkle_root,
                message_info.clone(),
                verification_session_pda,
            )
            .await;

        let _tx = match tx {
            Ok(tx) => tx,
            Err(err) => {
                let gateway_error = err.get_gateway_error().unwrap();
                assert_eq!(gateway_error, GatewayError::MessageAlreadyInitialised);
                continue;
            }
        };

        message_counter += 1;

        let destination_address =
            Pubkey::from_str(&message_info.leaf.message.destination_address).unwrap();

        // Assert PDA state for message approval
        let command_id = command_id(
            &message_info.leaf.message.cc_id.chain,
            &message_info.leaf.message.cc_id.id,
        );
        let (incoming_message_pda, incoming_message_pda_bump) =
            get_incoming_message_pda(&command_id);
        let (_, signing_pda_bump) =
            get_validate_message_signing_pda(destination_address, command_id);

        let account = metadata.incoming_message(incoming_message_pda).await;
        let expected_message = IncomingMessage::new(
            incoming_message_pda_bump,
            signing_pda_bump,
            MessageStatus::approved(),
            hash,
            message_info.leaf.message.payload_hash,
        );
        assert_eq!(account, expected_message);
    }

    assert_eq!(
        events_counter,
        messages_batch_two.len() - messages_batch_one.len(),
        "expected new unique events in the second batch"
    );
    assert_eq!(
        message_counter,
        messages_batch_two.len() - messages_batch_one.len(),
        "expected only unique messages from second batch to be processed"
    );
}

// the same message can only be approved once, subsequent calls will fail
#[tokio::test]
async fn fail_approvals_many_times_same_batch() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;

    let messages = make_messages(2);

    // verify the signatures
    let payload = Payload::Messages(Messages(messages.clone()));
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    // approve the messages initially
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

    // try to approve the messages again (will fail)
    let MerkleisedPayload::NewMessages { messages } = execute_data.payload_items.clone() else {
        unreachable!()
    };

    for message_info in messages {
        let tx = metadata
            .approve_message(
                execute_data.payload_merkle_root,
                message_info.clone(),
                verification_session_pda,
            )
            .await
            .unwrap_err();
        let gateway_error = tx.get_gateway_error().unwrap();
        assert_eq!(gateway_error, GatewayError::MessageAlreadyInitialised);
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
    for (merkle_root, leaves, proofs) in [
        (fake_payload_merkle_root, fake_leaves(), fake_proofs()),
        (fake_payload_merkle_root, fake_leaves(), valid_proofs()),
        (fake_payload_merkle_root, valid_leaves(), valid_proofs()),
        (payload_merkle_root, fake_leaves(), fake_proofs()),
        (payload_merkle_root, fake_leaves(), valid_proofs()),
        (payload_merkle_root, valid_leaves(), fake_proofs()),
    ] {
        for (leaf, proof) in leaves.into_iter().zip(proofs.into_iter()) {
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
    for (merkle_root, leaves, proofs) in [
        (fake_payload_merkle_root, fake_leaves(), fake_proofs()),
        (new_verifier_set_merkle_root, fake_leaves(), fake_proofs()),
    ] {
        for (leaf, proof) in leaves.into_iter().zip(proofs.into_iter()) {
            let new_message_info = MerkleisedMessage { leaf, proof };
            metadata
                .approve_message(merkle_root, new_message_info, verification_session_pda)
                .await
                .unwrap_err();
        }
    }
}

#[tokio::test]
async fn fails_to_approve_message_with_invalid_domain_separator() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;

    // Create a payload with messages signed by the registered verifier set
    let payload = Payload::Messages(Messages(make_messages(1)));
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);

    // Initialize and sign the payload session with registered verifier set
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    // Get the original message and modify its domain separator (simulating cross-chain replay attack)
    let MerkleisedPayload::NewMessages {
        messages: original_messages,
    } = execute_data.payload_items
    else {
        unreachable!();
    };

    let mut message_info = original_messages.into_iter().next().unwrap();

    // Modify the domain separator to simulate a cross-chain replay attack
    message_info.leaf.domain_separator[0] = message_info.leaf.domain_separator[0].wrapping_add(1);

    // Attempt to approve message with different domain separator
    let tx_result = metadata
        .approve_message(
            execute_data.payload_merkle_root,
            message_info,
            verification_session_pda,
        )
        .await
        .unwrap_err();

    // Should fail due to domain separator mismatch
    let gateway_error = tx_result.get_gateway_error().unwrap();
    assert_eq!(gateway_error, GatewayError::InvalidDomainSeparator);
}

/// Test that old (but still active) verifier sets can fully process a message approval cycle
#[tokio::test]
#[rstest::rstest]
#[case(1)]
#[case(3)]
#[case(10)]
async fn test_old_verifier_set_message_approval(#[case] rotation_count: usize) {
    // Setup with sufficient retention to keep old verifier sets active
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 55, 33])
        .previous_signers_retention(1 + rotation_count as u64) // Ensure old verifier sets remain active
        .build()
        .setup()
        .await;

    // Store the original verifier set for later testing
    let original_verifier_set = metadata.signers.clone();
    let mut current_verifier_set = original_verifier_set.clone();

    // Perform the specified number of signer rotations
    for i in 0..rotation_count {
        let weights: Vec<u128> = iter::repeat_with(|| rand::thread_rng().gen_range(50..200))
            .take(3)
            .collect();
        let new_verifier_set =
            make_verifier_set(&weights, (i + 1) as u64, metadata.domain_separator);

        // Perform signer rotation
        let (_verification_session_pda, rotate_result) = metadata
            .sign_session_and_rotate_signers(
                &current_verifier_set,
                &new_verifier_set.verifier_set(),
            )
            .await
            .unwrap(); // init signing session succeeded

        rotate_result.unwrap(); // signer rotation succeeded
        current_verifier_set = new_verifier_set;
    }

    // Now test that the original (old) verifier set can still process message approval
    let test_message = random_message();

    // Step 1: Initialize verification session with old verifier set
    let payload = Payload::Messages(Messages(vec![test_message.clone()]));
    let execute_data = metadata.construct_execute_data(&original_verifier_set, payload);

    // Step 2: Initialize and verify payload session manually
    let verification_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .expect("Should be able to initialize and verify with old verifier set");

    // Step 3: Extract the message to approve
    let MerkleisedPayload::NewMessages { messages } = execute_data.payload_items else {
        unreachable!("we constructed a message batch");
    };

    let message_to_approve = messages.into_iter().next().unwrap();

    // Step 4: Approve the message using the old verifier set
    metadata
        .approve_message(
            execute_data.payload_merkle_root,
            message_to_approve.clone(),
            verification_session_pda,
        )
        .await
        .expect("Old verifier set should be able to approve messages");

    // Step 5: Verify the message was properly approved
    let command_id = command_id(&test_message.cc_id.chain, &test_message.cc_id.id);
    let (incoming_message_pda, _) = get_incoming_message_pda(&command_id);

    let incoming_message = metadata.incoming_message(incoming_message_pda).await;
    assert_eq!(incoming_message.status, MessageStatus::approved());
    assert_eq!(incoming_message.payload_hash, test_message.payload_hash);

    // Additional verification: check the message hash matches what was used in the approval
    let expected_message_hash = message_to_approve.leaf.message.hash::<NativeHasher>();
    assert_eq!(incoming_message.message_hash, expected_message_hash);

    // Verify we used the correct (old) verifier set by checking the execute data
    let original_verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &original_verifier_set.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();
    assert_eq!(
        execute_data.signing_verifier_set_merkle_root,
        original_verifier_set_hash
    );
}
