use axelar_solana_encoding::hasher::NativeHasher;
use axelar_solana_encoding::types::verifier_set::verifier_set_hash;
use axelar_solana_gateway::get_gateway_root_config_pda;
use axelar_solana_gateway::state::signature_verification::SignatureVerification;
use axelar_solana_gateway_test_fixtures::gateway::{make_verifier_set, random_bytes};
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use bytemuck::Zeroable;
use solana_program_test::tokio;
use solana_sdk::instruction::InstructionError;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::TransactionError;

#[tokio::test]
async fn test_initialize_payload_verification_session() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup()
        .await;

    // Action
    let payload_merkle_root = random_bytes();
    let gateway_config_pda = get_gateway_root_config_pda().0;
    let signing_verifier_set_hash = metadata.init_gateway_config_verifier_set_data().hash;

    let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
        signing_verifier_set_hash,
    )
    .unwrap();
    let _tx_result = metadata.send_tx(&[ix]).await.unwrap();

    // Check PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &payload_merkle_root,
        &signing_verifier_set_hash,
    );

    let verification_session_account = metadata
        .try_get_account_no_checks(&verification_pda)
        .await
        .ok()
        .flatten()
        .expect("verification session PDA account should exist");

    assert_eq!(
        verification_session_account.owner,
        axelar_solana_gateway::ID
    );

    let session = metadata
        .signature_verification_session(verification_pda)
        .await;

    assert_eq!(session.bump, bump);
    let mut expected_verification = SignatureVerification::zeroed();
    expected_verification.signing_verifier_set_hash = signing_verifier_set_hash;
    assert_eq!(session.signature_verification, expected_verification);
}

#[tokio::test]
async fn test_cannot_initialize_pda_twice() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup()
        .await;

    // Action: First initialization
    let payload_merkle_root = random_bytes();
    let gateway_config_pda = get_gateway_root_config_pda().0;
    let signing_verifier_set_hash = metadata.init_gateway_config_verifier_set_data().hash;

    let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
        signing_verifier_set_hash,
    )
    .unwrap();
    let _tx_result = metadata.send_tx(&[ix]).await.unwrap();

    // Attempt to initialize the PDA a second time
    let ix_second = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
        signing_verifier_set_hash,
    )
    .unwrap();
    let tx_result_second = metadata.send_tx(&[ix_second]).await.unwrap_err();

    // Assert that the second initialization fails
    assert!(
        tx_result_second.result.is_err(),
        "Second initialization should fail"
    );
}

#[tokio::test]
async fn test_cannot_initialize_with_unknown_verifier_set() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup()
        .await;

    // Create an unknown verifier set that hasn't been registered with the gateway
    let unknown_verifier_set = make_verifier_set(&[100], 999, metadata.domain_separator);
    let unknown_verifier_set_hash = verifier_set_hash::<NativeHasher>(
        &unknown_verifier_set.verifier_set(),
        &metadata.domain_separator,
    )
    .unwrap();

    // Attempt to initialize payload verification session with unknown verifier set
    let payload_merkle_root = random_bytes();
    let gateway_config_pda = get_gateway_root_config_pda().0;
    let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
        unknown_verifier_set_hash,
    )
    .unwrap();

    let tx_result = metadata.send_tx(&[ix]).await.unwrap_err();

    // Verify the transaction fails with the expected error.
    // The verifier set tracker PDA doesn't exist for unknown verifier sets,
    // causing InsufficientFunds error when trying to access the non-existent account.
    match &tx_result.result {
        Err(TransactionError::InstructionError(_, InstructionError::InsufficientFunds)) => {}
        _ => panic!(
            "Expected InsufficientFunds error for unknown verifier set, got: {:?}",
            tx_result.result
        ),
    }
}
