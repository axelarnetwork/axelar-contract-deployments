use axelar_message_primitives::U256;
use axelar_solana_encoding::hasher::NativeHasher;
use axelar_solana_encoding::types::messages::Messages;
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::types::verifier_set::verifier_set_hash;
use axelar_solana_gateway::get_verifier_set_tracker_pda;
use axelar_solana_gateway::processor::{GatewayEvent, VerifierSetRotated};
use axelar_solana_gateway::state::verifier_set_tracker::VerifierSetTracker;
use axelar_solana_gateway_test_fixtures::gateway::{
    get_gateway_events, make_messages, make_verifier_set, random_bytes, random_message,
};
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

/// successfully process execute when there is 1 rotate signers commands
#[tokio::test]
async fn successfully_rotates_signers() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let new_verifier_set = make_verifier_set(&[500, 200], 1, metadata.domain_separator);
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let new_verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &new_verifier_set.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();
    let verification_session_account = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    let rotate_signers_ix = axelar_solana_gateway::instructions::rotate_signers(
        metadata.gateway_root_pda,
        verification_session_account,
        metadata.signers.verifier_set_tracker().0,
        new_verifier_set.verifier_set_tracker().0,
        metadata.payer.pubkey(),
        None,
        new_verifier_set_hash,
        new_verifier_set.verifier_set_tracker().1,
    )
    .unwrap();

    let tx_result = metadata
        .send_tx(&[
            // native digital signature verification won't work without bumping the compute budget.
            rotate_signers_ix,
        ])
        .await
        .unwrap();

    let new_epoch: U256 = 2u128.into();

    // - expected events
    let emitted_event = get_gateway_events(&tx_result).pop().unwrap();
    let GatewayEvent::VerifierSetRotated(emitted_event) = emitted_event else {
        panic!("unexpected event");
    };
    let expected_event = VerifierSetRotated {
        epoch: new_epoch,
        verifier_set_hash: new_verifier_set_hash,
    };
    assert_eq!(emitted_event, expected_event);

    // - signers have been updated
    let root_pda_data = metadata.gateway_confg(metadata.gateway_root_pda).await;
    assert_eq!(
        root_pda_data.auth_weighted.current_epoch(),
        new_epoch.clone()
    );
    // assert that the signer tracker pda has been initialized
    let verifier_set_tracker_data = metadata
        .verifier_set_tracker(new_verifier_set.verifier_set_tracker().0)
        .await;
    assert_eq!(
        verifier_set_tracker_data,
        VerifierSetTracker {
            bump: new_verifier_set.verifier_set_tracker().1,
            epoch: new_epoch,
            verifier_set_hash: new_verifier_set_hash
        }
    );
}

#[tokio::test]
async fn fail_when_approve_messages_payload_hash_is_used() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let messages = make_messages(1);
    let payload = Payload::Messages(Messages(messages.clone()));
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let verification_session_account = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    let verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &metadata.signers.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();
    let (new_vs_tracker_pda, new_vs_tracker_bump) = get_verifier_set_tracker_pda(random_bytes());

    let rotate_signers_ix = axelar_solana_gateway::instructions::rotate_signers(
        metadata.gateway_root_pda,
        verification_session_account,
        metadata.signers.verifier_set_tracker().0,
        new_vs_tracker_pda,
        metadata.payer.pubkey(),
        None,
        verifier_set_hash,
        new_vs_tracker_bump,
    )
    .unwrap();

    let tx_result = metadata
        .send_tx(&[
            // native digital signature verification won't work without bumping the compute budget.
            rotate_signers_ix,
        ])
        .await
        .unwrap_err();
    assert!(tx_result.result.is_err());
}

#[tokio::test]
async fn cannot_invoke_rotate_signers_without_respecting_minimum_delay() {
    // Setup
    let minimum_delay_seconds = 3;
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .minimum_rotate_signers_delay_seconds(minimum_delay_seconds)
        .build()
        .setup()
        .await;
    // after we set up the gateway, the minimum delay needs to be forwarded
    metadata.forward_time(minimum_delay_seconds as i64).await;

    // Action - rotate the signer set for the first time.
    let new_verifier_set = make_verifier_set(&[500, 200], 1, metadata.domain_separator);
    metadata
        .sign_session_and_rotate_signers(
            &metadata.signers.clone(),
            &new_verifier_set.verifier_set(),
        )
        .await
        .unwrap() // signing session succeeded
        .1
        .unwrap(); // signer rotation succeeded

    // Action - rotate the signer set for the second time.
    // this action does not wait for the minimum_delay_seconds, it should fail.
    let newer_verifier_set = make_verifier_set(&[444, 555], 333, metadata.domain_separator);
    let (signing_session_pda, rotate_signrs_tx_result) = metadata
        .sign_session_and_rotate_signers(&new_verifier_set, &newer_verifier_set.verifier_set())
        .await
        .unwrap(); // init signing session succeeded

    // Assert we are seeing the correct error message in tx logs.
    assert!(rotate_signrs_tx_result
        .unwrap_err()
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Command needs more time before being executed again",) }));

    // Action, forward time
    metadata.forward_time(minimum_delay_seconds as i64).await;

    // Action, rotate signers again after waiting the minimum delay.
    let tx = metadata
        .rotate_signers(
            &new_verifier_set,
            &newer_verifier_set.verifier_set(),
            signing_session_pda,
        )
        .await
        .unwrap();
    // Assert the rotate_signers transaction succeeded after waiting the time
    // required delay.
    assert!(tx.result.is_ok())
}

/// Ensure that we can use an old signer set to sign messages as long as the
/// operator also signed the `rotate_signers` ix
#[tokio::test]
async fn succeed_if_verifier_set_signed_by_old_verifier_set_and_submitted_by_the_operator() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .previous_signers_retention(2)
        .build()
        .setup()
        .await;

    // Action - rotate the signer set for the first time.
    // The signer set here will not be used in the following actions
    {
        let new_verifier_set =
            make_verifier_set(&[500, 200, 15, 555], 1, metadata.domain_separator);
        metadata
            .sign_session_and_rotate_signers(
                &metadata.signers.clone(),
                &new_verifier_set.verifier_set(),
            )
            .await
            .unwrap() // signing session succeeded
            .1
            .unwrap(); // signer rotation succeeded
    }

    // Action
    let new_verifier_set = make_verifier_set(&[500, 200], 2, metadata.domain_separator);
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let signing_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();
    let new_verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &new_verifier_set.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();
    let (new_vs_tracker_pda, new_vs_tracker_bump) =
        axelar_solana_gateway::get_verifier_set_tracker_pda(new_verifier_set_hash);
    let rotate_signers_ix = axelar_solana_gateway::instructions::rotate_signers(
        metadata.gateway_root_pda,
        signing_session_pda,
        metadata.signers.verifier_set_tracker().0,
        new_vs_tracker_pda,
        metadata.payer.pubkey(),
        Some(metadata.operator.pubkey()),
        new_verifier_set_hash,
        new_vs_tracker_bump,
    )
    .unwrap();

    let operator = metadata.operator.insecure_clone();
    let payer = metadata.payer.insecure_clone();
    let tx = metadata
        .send_tx_with_custom_signers(&[rotate_signers_ix], &[&operator, &payer])
        .await
        .unwrap();

    // Assert
    assert!(tx.result.is_ok());
    let new_epoch: U256 = 3_u128.into();
    let emitted_event = get_gateway_events(&tx).pop().unwrap();
    let GatewayEvent::VerifierSetRotated(emitted_event) = emitted_event else {
        panic!("unexpected event");
    };
    let expected_event = VerifierSetRotated {
        epoch: new_epoch,
        verifier_set_hash: new_verifier_set_hash,
    };
    assert_eq!(emitted_event, expected_event);

    // - signers have been updated
    let root_pda_data = metadata.gateway_confg(metadata.gateway_root_pda).await;
    assert_eq!(
        root_pda_data.auth_weighted.current_epoch(),
        new_epoch.clone()
    );
    let vs_tracker = metadata.verifier_set_tracker(new_vs_tracker_pda).await;
    assert_eq!(
        vs_tracker,
        VerifierSetTracker {
            bump: new_vs_tracker_bump,
            epoch: new_epoch,
            verifier_set_hash: new_verifier_set_hash
        }
    );
}

/// We use a different account in place of the expected operator to try and
/// rotate signers - but an on-chain check rejects his attempts
#[tokio::test]
async fn fail_if_provided_operator_is_not_the_real_operator_thats_stored_in_gateway_state() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .previous_signers_retention(2)
        .build()
        .setup()
        .await;

    // Action - rotate the signer set for the first time.
    // The signer set here will not be used in the following actions
    {
        let new_verifier_set =
            make_verifier_set(&[500, 200, 15, 555], 1, metadata.domain_separator);
        metadata
            .sign_session_and_rotate_signers(
                &metadata.signers.clone(),
                &new_verifier_set.verifier_set(),
            )
            .await
            .unwrap() // signing session succeeded
            .1
            .unwrap(); // signer rotation succeeded
    }

    // Action
    let fake_operator = Keypair::new();
    let new_verifier_set = make_verifier_set(&[500, 200], 2, metadata.domain_separator);
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let signing_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();
    let new_verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &new_verifier_set.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();
    let (new_vs_tracker_pda, new_vs_tracker_bump) =
        axelar_solana_gateway::get_verifier_set_tracker_pda(new_verifier_set_hash);
    let rotate_signers_ix = axelar_solana_gateway::instructions::rotate_signers(
        metadata.gateway_root_pda,
        signing_session_pda,
        metadata.signers.verifier_set_tracker().0,
        new_vs_tracker_pda,
        metadata.payer.pubkey(),
        Some(fake_operator.pubkey()),
        new_verifier_set_hash,
        new_vs_tracker_bump,
    )
    .unwrap();

    let payer = metadata.payer.insecure_clone();
    let tx = metadata
        .send_tx_with_custom_signers(&[rotate_signers_ix], &[&fake_operator, &payer])
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set",) }));
}

/// Ensure that the operator also need to explicitly sign the ix
#[tokio::test]
async fn fail_if_operator_only_passed_but_not_actual_signer() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .previous_signers_retention(2)
        .build()
        .setup()
        .await;

    // Action - rotate the signer set for the first time.
    // The signer set here will not be used in the following actions
    {
        let new_verifier_set =
            make_verifier_set(&[500, 200, 15, 555], 1, metadata.domain_separator);
        metadata
            .sign_session_and_rotate_signers(
                &metadata.signers.clone(),
                &new_verifier_set.verifier_set(),
            )
            .await
            .unwrap() // signing session succeeded
            .1
            .unwrap(); // signer rotation succeeded
    }

    // Action
    let new_verifier_set = make_verifier_set(&[500, 200], 2, metadata.domain_separator);
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    let signing_session_pda = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();
    let new_verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &new_verifier_set.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();
    let (new_vs_tracker_pda, new_vs_tracker_bump) =
        axelar_solana_gateway::get_verifier_set_tracker_pda(new_verifier_set_hash);
    let mut rotate_signers_ix = axelar_solana_gateway::instructions::rotate_signers(
        metadata.gateway_root_pda,
        signing_session_pda,
        metadata.signers.verifier_set_tracker().0,
        new_vs_tracker_pda,
        metadata.payer.pubkey(),
        Some(metadata.operator.pubkey()),
        new_verifier_set_hash,
        new_vs_tracker_bump,
    )
    .unwrap();
    // set the 'operator' as non-signer to get the tx in, otherwise Solana will
    // reject for missing signatures
    rotate_signers_ix.accounts.last_mut().unwrap().is_signer = false;

    let tx = metadata.send_tx(&[rotate_signers_ix]).await.unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}

/// disallow rotate signers if any other signer set besides the most recent
/// epoch signed the proof
#[tokio::test]
async fn fail_if_rotate_signers_signed_by_old_verifier_set() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .previous_signers_retention(100) // this ensures that all verifier sets are valid
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;

    {
        let mut preveious = metadata.signers.clone();
        for i in 0..5 {
            // Action - rotate the signer set for the first time.
            let new_verifier_set =
                make_verifier_set(&[i, 200], i as u64, metadata.domain_separator);
            metadata
                .sign_session_and_rotate_signers(&preveious, &new_verifier_set.verifier_set())
                .await
                .unwrap() // signing session succeeded
                .1
                .unwrap(); // signer rotation succeeded
            preveious = new_verifier_set;
        }
    }

    // Action - rotate the signer set for the second time.
    // note: we use the original signers (now considered an old verifier set)
    let new_verifier_set = make_verifier_set(&[444, 555], 333, metadata.domain_separator);
    let (_signing_session_pda, rotate_signrs_tx_result) = metadata
        .sign_session_and_rotate_signers(
            &metadata.signers.clone(),
            &new_verifier_set.verifier_set(),
        )
        .await
        .unwrap(); // init signing session succeeded

    // Assert we are seeing the correct error message in tx logs.
    assert!(rotate_signrs_tx_result
        .unwrap_err()
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}

// new verifier set can approve messages
#[tokio::test]
async fn new_verifier_set_can_approve_messages_while_respecting_signer_retention() {
    // Setup
    let previous_signer_retention = 3;
    let mut metadata = SolanaAxelarIntegration::builder()
        .previous_signers_retention(previous_signer_retention)
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;

    let mut new_signer_sets = vec![];
    let mut previous = metadata.signers.clone();
    for i in 0..previous_signer_retention {
        // Action - rotate the signer set for the first time.
        let new_verifier_set = make_verifier_set(&[1, 200], i, metadata.domain_separator);
        new_signer_sets.push(new_verifier_set.clone());
        metadata
            .sign_session_and_rotate_signers(&previous, &new_verifier_set.verifier_set())
            .await
            .unwrap() // signing session succeeded
            .1
            .unwrap(); // signer rotation succeeded

        // confidence check: can approve messages
        metadata
            .sign_session_and_approve_messages(&new_verifier_set, &[random_message()])
            .await
            .unwrap();

        // store the last signer
        previous = new_verifier_set;
    }

    assert_eq!(
        new_signer_sets.len() as u64,
        previous_signer_retention,
        "new signer sets"
    );

    // Action: can still approve messages, except the initial one
    for signer_set in new_signer_sets {
        metadata
            .sign_session_and_approve_messages(&signer_set, &[random_message()])
            .await
            .unwrap();
    }

    // Check: signer set that falls out of bounds cannot approve messages
    metadata
        .sign_session_and_approve_messages(&metadata.signers.clone(), &[random_message()])
        .await
        .unwrap_err();
}
