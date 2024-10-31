use axelar_rkyv_encoding::test_fixtures::{
    random_bytes, random_ecdsa_signature, random_ed25519_signature,
};
use axelar_rkyv_encoding::types::PublicKey;
use axelar_solana_gateway::state::signature_verification_pda::SignatureVerificationSessionData;
use solana_program_test::tokio;
use solana_sdk::account::ReadableAccount;
use solana_sdk::compute_budget::ComputeBudgetInstruction;

use crate::setup::{SignatureVerificationInput, TestSuite};

#[tokio::test]
async fn test_verify_one_signature() {
    // Setup
    let payload_merkle_root = random_bytes();

    let TestSuite {
        mut runner,
        gateway_config_pda,
        initial_verifier_set_tracker_pda,
        verification_inputs_iterator,
        ..
    } = TestSuite::new_with_pending_payloads(&[payload_merkle_root]).await;

    let SignatureVerificationInput {
        leaf,
        proof,
        signature,
    } = verification_inputs_iterator
        .for_payload_root(payload_merkle_root)
        .next()
        .unwrap();

    // Verify the signature
    let ix = axelar_solana_gateway::instructions::verify_signature(
        gateway_config_pda,
        initial_verifier_set_tracker_pda,
        payload_merkle_root,
        leaf,
        proof,
        signature,
    )
    .unwrap();

    let tx_result = runner
        .send_tx_with_metadata(&[
            // native digital signature verification won't work without bumping the compute budget.
            ComputeBudgetInstruction::set_compute_unit_limit(600_000),
            ix,
        ])
        .await;
    assert!(tx_result.result.is_ok());

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &gateway_config_pda,
        &payload_merkle_root,
    );

    let verification_session_account = runner
        .banks_client
        .get_account(verification_pda)
        .await
        .ok()
        .flatten()
        .expect("verification session PDA account should exist");

    assert_eq!(
        verification_session_account.owner,
        axelar_solana_gateway::ID
    );

    let mut buffer = [0u8; SignatureVerificationSessionData::LEN];
    buffer.copy_from_slice(verification_session_account.data());
    let session: SignatureVerificationSessionData = bytemuck::cast(buffer);

    assert_eq!(session.bump, bump);

    // Only the first slot should be set
    let mut slots = session.signature_verification.slots();
    assert_eq!(slots.next(), Some(true), "first slot should be set");
    assert!(slots.all(|slot| !slot), "remaining slots should be unset");
    assert!(
        !session.signature_verification.is_valid(),
        "session should not be valid after just a single signature is verified"
    );
}

#[tokio::test]
async fn test_verify_all_signatures() {
    // Setup
    let payload_merkle_root = random_bytes();

    let TestSuite {
        mut runner,
        gateway_config_pda,
        initial_verifier_set_tracker_pda,
        verification_inputs_iterator,
        ..
    } = TestSuite::new_with_pending_payloads(&[payload_merkle_root]).await;

    let mut num_signatures = 0;
    for SignatureVerificationInput {
        leaf,
        proof,
        signature,
    } in verification_inputs_iterator.for_payload_root(payload_merkle_root)
    {
        // Verify the signature
        let ix = axelar_solana_gateway::instructions::verify_signature(
            gateway_config_pda,
            initial_verifier_set_tracker_pda,
            payload_merkle_root,
            leaf,
            proof,
            signature,
        )
        .unwrap();
        let tx_result = runner
            .send_tx_with_metadata(&[
                // native digital signature verification won't work without bumping the compute
                // budget.
                ComputeBudgetInstruction::set_compute_unit_limit(600_000),
                ix,
            ])
            .await;
        assert!(tx_result.result.is_ok());
        num_signatures += 1;
    }

    // Check that the PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &gateway_config_pda,
        &payload_merkle_root,
    );

    let verification_session_account = runner
        .banks_client
        .get_account(verification_pda)
        .await
        .ok()
        .flatten()
        .expect("verification session PDA account should exist");

    assert_eq!(
        verification_session_account.owner,
        axelar_solana_gateway::ID
    );

    let mut buffer = [0u8; SignatureVerificationSessionData::LEN];
    buffer.copy_from_slice(verification_session_account.data());
    let session: SignatureVerificationSessionData = bytemuck::cast(buffer);

    assert_eq!(session.bump, bump);
    let mut slots = session.signature_verification.slots();
    assert!(
        slots.by_ref().take(num_signatures).all(|slot| slot),
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

    let TestSuite {
        mut runner,
        gateway_config_pda,
        initial_verifier_set_tracker_pda,
        verification_inputs_iterator,
        ..
    } = TestSuite::new_with_pending_payloads(&[payload_merkle_root]).await;

    let SignatureVerificationInput { leaf, proof, .. } = verification_inputs_iterator
        .for_payload_root(payload_merkle_root)
        .next()
        .unwrap();

    // Use a random signature:
    let signature = match &leaf.signer_pubkey {
        PublicKey::Secp256k1(_) => random_ecdsa_signature(),
        PublicKey::Ed25519(_) => random_ed25519_signature(),
    };

    // Send the transaction
    let ix = axelar_solana_gateway::instructions::verify_signature(
        gateway_config_pda,
        initial_verifier_set_tracker_pda,
        payload_merkle_root,
        leaf,
        proof,
        signature,
    )
    .unwrap();
    let tx_result = runner
        .send_tx_with_metadata(&[
            // native digital signature verification won't work without bumping the compute budget.
            ComputeBudgetInstruction::set_compute_unit_limit(600_000),
            ix,
        ])
        .await;
    assert!(tx_result.result.is_err());

    // Check logs to assert the failure
    assert!(tx_result
        .metadata
        .expect("transaction should've returned with metadata")
        .log_messages
        .iter()
        .any(|log| {
            log.to_lowercase()
                .contains("digital signature verification failed")
        }));

    // Check that the verification PDA data wasn't altered
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &gateway_config_pda,
        &payload_merkle_root,
    );

    let verification_session_account = runner
        .banks_client
        .get_account(verification_pda)
        .await
        .ok()
        .flatten()
        .expect("verification session PDA account should exist");

    assert_eq!(
        verification_session_account.owner,
        axelar_solana_gateway::ID
    );

    let mut buffer = [0u8; SignatureVerificationSessionData::LEN];
    buffer.copy_from_slice(verification_session_account.data());
    let session: SignatureVerificationSessionData = bytemuck::cast(buffer);

    assert_eq!(session.bump, bump);
    assert!(
        session.signature_verification.slots().all(|slot| !slot),
        "all slots should be unset"
    );
    assert!(
        !session.signature_verification.is_valid(),
        "session should not be valid"
    );
}
