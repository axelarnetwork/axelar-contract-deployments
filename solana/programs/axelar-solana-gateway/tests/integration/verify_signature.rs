use axelar_rkyv_encoding::test_fixtures::{
    random_bytes, random_ecdsa_signature, random_ed25519_signature,
};
use axelar_rkyv_encoding::types::PublicKey;
use axelar_solana_gateway_test_fixtures::gateway::make_verifier_set;
use axelar_solana_gateway_test_fixtures::test_signer::SignatureVerificationInput;
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use solana_program_test::tokio;
use solana_sdk::compute_budget::ComputeBudgetInstruction;

#[tokio::test]
#[rstest::rstest]
#[case(vec![42])]
#[case(vec![42, 43])]
async fn test_verify_one_signature(#[case] initial_signer_weights: Vec<u128>) {
    // Setup
    let payload_merkle_root = random_bytes();
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(initial_signer_weights.clone())
        .build()
        .setup()
        .await;

    metadata
        .initialize_payload_verification_session(metadata.gateway_root_pda, payload_merkle_root)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let vs_iterator = metadata
        .signers
        .init_signing_session(&metadata.signers.verifier_set());
    let mut signer_verification_input = vs_iterator.for_payload_root(payload_merkle_root);
    let first_signer_verification_input = signer_verification_input
        .next()
        .expect("guaranteed to have 1 item");

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        payload_merkle_root,
        first_signer_verification_input.verifier_set_leaf,
        first_signer_verification_input.verifier_set_proof,
        first_signer_verification_input.signature,
    )
    .unwrap();

    metadata
        .send_tx(&[
            // native digital signature verification won't work without bumping the compute budget.
            ComputeBudgetInstruction::set_compute_unit_limit(250_000),
            ix,
        ])
        .await
        .unwrap();

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &metadata.gateway_root_pda,
        &payload_merkle_root,
    );

    let session = metadata
        .signature_verification_session(verification_pda)
        .await;
    assert_eq!(session.bump, bump);

    // Only the first slot should be set
    let mut slots = session.signature_verification.slots_iter();
    assert_eq!(slots.next(), Some(true), "first slot should be set");
    assert!(slots.all(|slot| !slot), "remaining slots should be unset");
    let only_single_signer = initial_signer_weights.len() == 1;
    assert_eq!(
        session.signature_verification.is_valid(), only_single_signer,
        "session should be valid after just a single signature verification if we have a single signer in the set"
    );
}

#[tokio::test]
async fn test_verify_all_signatures() {
    // Setup
    let payload_merkle_root = random_bytes();
    let amount_of_signers = 64;
    let init_signer_weights = vec![42; amount_of_signers];
    let mut metadata = SolanaAxelarIntegration::builder()
        // 64 signers
        .initial_signer_weights(init_signer_weights)
        .build()
        .setup()
        .await;

    metadata
        .initialize_payload_verification_session(metadata.gateway_root_pda, payload_merkle_root)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let vs_iterator = metadata
        .signers
        .init_signing_session(&metadata.signers.verifier_set());
    let signer_verification_input = vs_iterator.for_payload_root(payload_merkle_root);

    for SignatureVerificationInput {
        verifier_set_leaf,
        verifier_set_proof,
        signature,
    } in signer_verification_input
    {
        // Verify the signature
        let ix = axelar_solana_gateway::instructions::verify_signature(
            metadata.gateway_root_pda,
            verifier_set_tracker_pda,
            payload_merkle_root,
            verifier_set_leaf,
            verifier_set_proof,
            signature,
        )
        .unwrap();
        metadata
            .send_tx(&[
                ComputeBudgetInstruction::set_compute_unit_limit(250_000),
                ix,
            ])
            .await
            .unwrap();
    }

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &metadata.gateway_root_pda,
        &payload_merkle_root,
    );

    let session = metadata
        .signature_verification_session(verification_pda)
        .await;

    assert_eq!(session.bump, bump);
    let mut slots = session.signature_verification.slots_iter();
    assert!(
        slots.by_ref().take(amount_of_signers).all(|slot| slot),
        "slot for verified signatures should be set"
    );
    assert!(slots.all(|slot| !slot), "remaining slots should be unset");
    assert!(
        session.signature_verification.is_valid(),
        "session should be valid after all signatures are verified"
    );
}

#[tokio::test]
async fn test_fails_to_verify_bad_signature() {
    // Setup
    let payload_merkle_root = random_bytes();
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42; 10])
        .build()
        .setup()
        .await;

    metadata
        .initialize_payload_verification_session(metadata.gateway_root_pda, payload_merkle_root)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let vs_iterator = metadata
        .signers
        .init_signing_session(&metadata.signers.verifier_set());
    let mut signer_verification_input = vs_iterator.for_payload_root(payload_merkle_root);
    let first_signer_verification_input = signer_verification_input
        .next()
        .expect("guaranteed to have 1 item");
    let invalid_signature = match &first_signer_verification_input
        .verifier_set_leaf
        .signer_pubkey
    {
        PublicKey::Secp256k1(_) => random_ecdsa_signature(),
        PublicKey::Ed25519(_) => random_ed25519_signature(),
    };

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        payload_merkle_root,
        first_signer_verification_input.verifier_set_leaf,
        first_signer_verification_input.verifier_set_proof,
        invalid_signature,
    )
    .unwrap();
    let tx_result = metadata
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(250_000),
            ix,
        ])
        .await
        .unwrap_err();
    assert!(tx_result
        .metadata
        .expect("transaction should've returned with metadata")
        .log_messages
        .iter()
        .any(|log| {
            log.to_lowercase()
                .contains("digital signature verification failed")
        }));
}

#[tokio::test]
async fn test_fails_to_verify_signature_for_different_merkle_root() {
    // Setup
    let payload_merkle_root = random_bytes();
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42; 10])
        .build()
        .setup()
        .await;

    metadata
        .initialize_payload_verification_session(metadata.gateway_root_pda, payload_merkle_root)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let vs_iterator = metadata
        .signers
        .init_signing_session(&metadata.signers.verifier_set());
    let mut signer_verification_input = vs_iterator.for_payload_root(payload_merkle_root);
    let first_signer_verification_input = signer_verification_input
        .next()
        .expect("guaranteed to have 1 item");

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        random_bytes(), // <- this is the failure culprit
        first_signer_verification_input.verifier_set_leaf,
        first_signer_verification_input.verifier_set_proof,
        first_signer_verification_input.signature,
    )
    .unwrap();
    let tx_result = metadata
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(250_000),
            ix,
        ])
        .await
        .unwrap_err();
    assert!(tx_result
        .metadata
        .expect("transaction should've returned with metadata")
        .log_messages
        .iter()
        .any(|log| {
            log.to_lowercase()
                .contains("session account data is corrupt")
        }));
}

#[tokio::test]
async fn test_large_weight_will_validate_whole_batch() {
    // Setup
    let payload_merkle_root = random_bytes();
    let large_weight = 100;
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![1, 1, large_weight])
        .custom_quorum(large_weight)
        .build()
        .setup()
        .await;

    metadata
        .initialize_payload_verification_session(metadata.gateway_root_pda, payload_merkle_root)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let vs_iterator = metadata
        .signers
        .init_signing_session(&metadata.signers.verifier_set());
    let mut signer_verification_input = vs_iterator.for_payload_root(payload_merkle_root);
    let large_weight_item = signer_verification_input
        .find(|x| x.verifier_set_leaf.element.signer_weight == large_weight)
        .expect("guaranteed to be in the set");

    // Verify the signature
    dbg!(&large_weight_item.verifier_set_leaf.element);
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        payload_merkle_root,
        large_weight_item.verifier_set_leaf,
        large_weight_item.verifier_set_proof,
        large_weight_item.signature,
    )
    .unwrap();

    let _tx_result = metadata
        .send_tx(&[
            // native digital signature verification won't work without bumping the compute budget.
            ComputeBudgetInstruction::set_compute_unit_limit(250_000),
            ix,
        ])
        .await
        .unwrap();

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &metadata.gateway_root_pda,
        &payload_merkle_root,
    );

    let session = metadata
        .signature_verification_session(verification_pda)
        .await;
    dbg!(&session);

    assert_eq!(session.bump, bump);

    assert!(
        session.signature_verification.is_valid(),
        "session should be valid because the single signer weight is large enough"
    );
    let slots = session.signature_verification.slots();
    assert!(
        slots[large_weight_item.verifier_set_leaf.element.position as usize],
        "first slot should not be set"
    );
}

#[tokio::test]
async fn fail_verification_if_non_registered_verifier_set_signed_batch() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .previous_signers_retention(2)
        .build()
        .setup()
        .await;
    // generate a new random verifier set to be used (do not register it)
    let new_random_verifier_set = make_verifier_set(&[11], 1, metadata.domain_separator);
    let new_verifier_set = make_verifier_set(&[500, 200], 1, metadata.domain_separator);

    // using `initial_singers` to sign the message which is the cause of the
    // failure
    let tx = metadata
        .init_payload_session_and_sign(
            &new_random_verifier_set.clone(),
            new_verifier_set.verifier_set().payload_hash(),
        )
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Invalid VerifierSetTracker PDA") }));
}

#[tokio::test]
async fn fail_signatures_if_quorum_not_met() {
    // Setup
    let very_large_quorum = 10;
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![1])
        .custom_quorum(very_large_quorum)
        .build()
        .setup()
        .await;
    let new_verifier_set = make_verifier_set(&[1, 1], 1, metadata.domain_separator);

    // Action
    let tx = metadata
        .init_payload_session_and_sign(
            &metadata.signers.clone(),
            new_verifier_set.verifier_set().payload_hash(),
        )
        .await
        .unwrap();

    // Assert
    let session = metadata.signature_verification_session(tx).await;
    assert!(!session.signature_verification.is_valid());
}
