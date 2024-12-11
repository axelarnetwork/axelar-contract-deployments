use axelar_solana_encoding::types::messages::Messages;
use axelar_solana_encoding::types::payload::Payload;
use axelar_solana_gateway::get_gateway_root_config_pda;
use axelar_solana_gateway::state::signature_verification::SignatureVerification;
use axelar_solana_gateway_test_fixtures::gateway::{
    make_messages, make_verifier_set, random_bytes,
};
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

#[tokio::test]
async fn test_same_payload_can_be_signed_by_multiple_verifier_sets_and_be_initialised() {
    // Setup
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup()
        .await;

    let signers_a = make_verifier_set(&[500, 200], 1, metadata.domain_separator);
    let signers_b = make_verifier_set(&[500, 23], 101, metadata.domain_separator);

    let messages = make_messages(5);
    let payload = Payload::Messages(Messages(messages.clone()));
    let execute_data_a = metadata.construct_execute_data(&signers_a, payload.clone());
    let execute_data_b = metadata.construct_execute_data(&signers_b, payload);

    for execute_data in [execute_data_a, execute_data_b] {
        let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
            metadata.payer.pubkey(),
            metadata.gateway_root_pda,
            execute_data.payload_merkle_root,
        )
        .unwrap();
        let _tx_result = metadata.send_tx(&[ix]).await.unwrap();
    }
}
