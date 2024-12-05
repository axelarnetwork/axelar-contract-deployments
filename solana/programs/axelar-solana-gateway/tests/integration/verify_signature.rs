use std::sync::Arc;

use axelar_solana_encoding::types::messages::Messages;
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
use axelar_solana_gateway::state::signature_verification::verify_ecdsa_signature;
use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_gateway_test_fixtures::gateway::{
    make_verifier_set, random_bytes, random_message,
};
use axelar_solana_gateway_test_fixtures::test_signer::{random_ecdsa_keypair, SigningVerifierSet};
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use solana_program_test::tokio;
use solana_sdk::compute_budget::ComputeBudgetInstruction;

#[tokio::test]
#[rstest::rstest]
#[case(vec![42], Messages(vec![random_message()]))]
#[case(vec![42, 43], Messages(vec![random_message()]))]
async fn test_verify_one_signature(
    #[case] initial_signer_weights: Vec<u128>,
    #[case] messages: Messages,
) {
    // Setup

    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(initial_signer_weights.clone())
        .build()
        .setup()
        .await;

    let payload = Payload::Messages(messages);
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    metadata
        .initialize_payload_verification_session(&execute_data)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let leaf_info = execute_data.signing_verifier_set_leaves.first().unwrap();

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        execute_data.payload_merkle_root,
        leaf_info.clone(),
    )
    .unwrap();

    metadata
        .send_tx(&[
            // native digital signature verification won't work without bumping the compute budget.
            ComputeBudgetInstruction::set_compute_unit_limit(260_000),
            ix,
        ])
        .await
        .unwrap();

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &metadata.gateway_root_pda,
        &execute_data.payload_merkle_root,
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
    let messages = Messages(vec![random_message(); 5]);
    let payload = Payload::Messages(messages);
    let amount_of_signers = 64;
    let init_signer_weights = vec![42; amount_of_signers];
    let mut metadata = SolanaAxelarIntegration::builder()
        // 64 signers
        .initial_signer_weights(init_signer_weights)
        .build()
        .setup()
        .await;
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);

    metadata
        .initialize_payload_verification_session(&execute_data)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;

    for verifier_set_leaf in execute_data.signing_verifier_set_leaves {
        // Verify the signature
        let ix = axelar_solana_gateway::instructions::verify_signature(
            metadata.gateway_root_pda,
            verifier_set_tracker_pda,
            execute_data.payload_merkle_root,
            verifier_set_leaf,
        )
        .unwrap();
        metadata
            .send_tx(&[
                ComputeBudgetInstruction::set_compute_unit_limit(260_000),
                ix,
            ])
            .await
            .unwrap();
    }

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &metadata.gateway_root_pda,
        &execute_data.payload_merkle_root,
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
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42; 10])
        .build()
        .setup()
        .await;
    let payload = Payload::Messages(Messages(vec![random_message(); 5]));
    let mut execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);

    metadata
        .initialize_payload_verification_session(&execute_data)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let leaf_info = execute_data.signing_verifier_set_leaves.get_mut(0).unwrap();
    match &mut leaf_info.signature {
        Signature::EcdsaRecoverable(data) => {
            *data = random_bytes();
        }
        Signature::Ed25519(data) => {
            *data = random_bytes();
        }
    };

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        execute_data.payload_merkle_root,
        leaf_info.clone(),
    )
    .unwrap();
    let tx_result = metadata
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(260_000),
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
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42; 10])
        .build()
        .setup()
        .await;
    let payload = Payload::Messages(Messages(vec![random_message(); 5]));
    let mut execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
    metadata
        .initialize_payload_verification_session(&execute_data)
        .await
        .unwrap();
    let leaf_info = execute_data.signing_verifier_set_leaves.get_mut(0).unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;

    let random_valid_merkle_root = {
        let payload = Payload::Messages(Messages(vec![random_message(); 5]));
        let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);
        metadata
            .initialize_payload_verification_session(&execute_data)
            .await
            .unwrap();
        execute_data.payload_merkle_root
    };

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        random_valid_merkle_root, // <- this is the failure culprit
        leaf_info.clone(),
    )
    .unwrap();
    let tx_result = metadata
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(260_000),
            ix,
        ])
        .await
        .unwrap_err();

    assert!(tx_result
        .find_log("Digital signature verification failed")
        .is_some());
}

#[tokio::test]
async fn test_large_weight_will_validate_whole_batch() {
    // Setup
    let large_weight = 100;
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![1, 1, large_weight])
        .custom_quorum(large_weight)
        .build()
        .setup()
        .await;
    let payload = Payload::Messages(Messages(vec![random_message(); 5]));
    let verifier_set = metadata.signers.verifier_set();
    let signer_set_with_only_large_weight = {
        let signer = metadata
            .signers
            .signers
            .iter()
            .find(|x| x.weight == large_weight)
            .unwrap()
            .clone();
        SigningVerifierSet {
            signers: Arc::from_iter([signer]),
            nonce: verifier_set.nonce,
            quorum: verifier_set.quorum,
            domain_separator: metadata.domain_separator,
        }
    };
    let execute_data = metadata.construct_execute_data_with_custom_verifier_set(
        &signer_set_with_only_large_weight,
        &verifier_set,
        payload,
    );

    metadata
        .initialize_payload_verification_session(&execute_data)
        .await
        .unwrap();
    let verifier_set_tracker_pda = metadata.signers.verifier_set_tracker().0;
    let large_wetight_leaf = execute_data
        .signing_verifier_set_leaves
        .iter()
        .find(|x| x.leaf.signer_weight == large_weight)
        .expect("guaranteed to be in the set");

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        metadata.gateway_root_pda,
        verifier_set_tracker_pda,
        execute_data.payload_merkle_root,
        large_wetight_leaf.clone(),
    )
    .unwrap();

    let _tx_result = metadata
        .send_tx(&[
            // native digital signature verification won't work without bumping the compute budget.
            ComputeBudgetInstruction::set_compute_unit_limit(260_000),
            ix,
        ])
        .await
        .unwrap();

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &metadata.gateway_root_pda,
        &execute_data.payload_merkle_root,
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
        slots[large_wetight_leaf.leaf.position as usize],
        "large weight leaf should be set"
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

    // using `new_random_verifier_set` to sign the message which is the cause of the
    // failure (should be metadata.signers)
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&new_random_verifier_set, payload);

    let tx = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap_err();

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("account does not have enough lamports") }));
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
    let payload = Payload::NewVerifierSet(new_verifier_set.verifier_set());
    let execute_data = metadata.construct_execute_data(&metadata.signers.clone(), payload);

    // Action
    let tx = metadata
        .init_payload_session_and_verify(&execute_data)
        .await
        .unwrap();

    // Assert
    let session = metadata.signature_verification_session(tx).await;
    assert!(!session.signature_verification.is_valid());
}

#[test]
fn can_verify_signatures_with_ecrecover_recovery_id() {
    let (keypair, pubkey) = random_ecdsa_keypair();
    let message_hash = [42; 32];
    let signature = keypair.sign(&message_hash);
    let Signature::EcdsaRecoverable(mut signature) = signature else {
        panic!("unexpected signature type");
    };
    signature[64] += 27;
    let PublicKey::Secp256k1(pubkey) = pubkey else {
        panic!("unexpected pubkey type");
    };

    let is_valid = verify_ecdsa_signature(&pubkey, &signature, &message_hash);
    assert!(is_valid);
}

#[test]
fn can_verify_signatures_with_standard_recovery_id() {
    let (keypair, pubkey) = random_ecdsa_keypair();
    let message_hash = [42; 32];
    let signature = keypair.sign(&message_hash);
    let Signature::EcdsaRecoverable(signature) = signature else {
        panic!("unexpected signature type");
    };
    assert!((0_u8..=3_u8).contains(&signature[64]));
    let PublicKey::Secp256k1(pubkey) = pubkey else {
        panic!("unexpected pubkey type");
    };

    let is_valid = verify_ecdsa_signature(&pubkey, &signature, &message_hash);
    assert!(is_valid);
}
