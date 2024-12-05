use axelar_solana_gateway::get_gateway_root_config_pda;
use axelar_solana_gateway::state::signature_verification::SignatureVerification;
use axelar_solana_gateway_test_fixtures::gateway::random_bytes;
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use bytemuck::Zeroable;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

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

    let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
    )
    .unwrap();
    let _tx_result = metadata.send_tx(&[ix]).await.unwrap();

    // Check PDA contains the expected data
    let (verification_pda, bump) = axelar_solana_gateway::get_signature_verification_pda(
        &gateway_config_pda,
        &payload_merkle_root,
    );

    let verification_session_account = metadata
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

    let session = metadata
        .signature_verification_session(verification_pda)
        .await;

    assert_eq!(session.bump, bump);
    assert_eq!(
        session.signature_verification,
        SignatureVerification::zeroed()
    );
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

    let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
    )
    .unwrap();
    let _tx_result = metadata.send_tx(&[ix]).await.unwrap();

    // Attempt to initialize the PDA a second time
    let ix_second = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        metadata.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
    )
    .unwrap();
    let tx_result_second = metadata.send_tx(&[ix_second]).await.unwrap_err();

    // Assert that the second initialization fails
    assert!(
        tx_result_second.result.is_err(),
        "Second initialization should fail"
    );
}
