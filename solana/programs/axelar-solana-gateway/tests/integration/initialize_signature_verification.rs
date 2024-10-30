use axelar_rkyv_encoding::test_fixtures::random_bytes;
use axelar_solana_gateway::state::signature_verification::SignatureVerification;
use axelar_solana_gateway::state::signature_verification_pda::SignatureVerificationSessionData;
use bytemuck::Zeroable;
use solana_program_test::tokio;
use solana_sdk::account::ReadableAccount;
use solana_sdk::signer::Signer;

use crate::setup::TestSuite;

#[tokio::test]
async fn test_initialize_payload_verification_session() {
    // Setup
    let TestSuite {
        mut runner,
        gateway_config_pda,
        ..
    } = TestSuite::new().await;

    // Action
    let payload_merkle_root = random_bytes();

    let ix = axelar_solana_gateway::instructions::initialize_payload_verification_session(
        runner.payer.pubkey(),
        gateway_config_pda,
        payload_merkle_root,
    )
    .unwrap();
    let tx_result = runner.send_tx_with_metadata(&[ix]).await;
    assert!(tx_result.result.is_ok());

    // Check PDA contains the expected data

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
    assert_eq!(
        session.signature_verification,
        SignatureVerification::zeroed()
    );
}
