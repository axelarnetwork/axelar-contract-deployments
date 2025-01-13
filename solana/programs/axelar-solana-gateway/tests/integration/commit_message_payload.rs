use crate::initialize_message_payload::{initialize_message_payload_pda, message_to_command_id};
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use axelar_solana_gateway_test_fixtures::gateway::{random_bytes, random_message};
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use sha3::{Digest, Keccak256};
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

#[tokio::test]
#[allow(clippy::as_conversions)]
async fn successfully_commit_message_payload_pda() {
    const PAYLOAD_SIZE: usize = 1024;

    // Setup: Test runner
    let mut runner = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let gateway_root_pda = runner.gateway_root_pda;

    // Setup: Patch the input message to use a valid payload hash.
    let message_payload_bytes_to_write = random_bytes::<PAYLOAD_SIZE>();
    let message = {
        let mut message = random_message();
        message.payload_hash = Keccak256::digest(message_payload_bytes_to_write).into();
        message
    };

    // Setup: Send an instruction to initialize the message payload PDA account
    initialize_message_payload_pda(&mut runner, &message, PAYLOAD_SIZE as u64).await;

    // Setup: Build and send an instruction to write the message payload bytes
    let command_id = message_to_command_id(&message);
    let (message_payload_pda, _) = axelar_solana_gateway::find_message_payload_pda(
        gateway_root_pda,
        command_id,
        runner.payer.pubkey(),
    );
    let write_ix = axelar_solana_gateway::instructions::write_message_payload(
        gateway_root_pda,
        runner.payer.pubkey(),
        command_id,
        &message_payload_bytes_to_write,
        0,
    )
    .unwrap();
    let write_tx = runner.send_tx(&[write_ix]).await.unwrap();
    assert!(write_tx.result.is_ok());

    // Action: Build and send an instruction to commit
    let ix = axelar_solana_gateway::instructions::commit_message_payload(
        gateway_root_pda,
        runner.payer.pubkey(),
        command_id,
    )
    .unwrap();
    let tx = runner.send_tx(&[ix]).await.unwrap();
    assert!(tx.result.is_ok());

    // Assert that the message payload PDA contains the expected payload hash
    assert_message_payload_has_the_expected_payload_hash(
        &mut runner,
        message_payload_pda,
        message.payload_hash,
    )
    .await;
}

async fn assert_message_payload_has_the_expected_payload_hash(
    runner: &mut SolanaAxelarIntegrationMetadata,
    message_payload_pda: Pubkey,
    expected_payload_hash: [u8; 32],
) {
    let account = runner
        .get_account(&message_payload_pda, &axelar_solana_gateway::ID)
        .await;
    let parsed: ImmutMessagePayload<'_> = account
        .data
        .as_slice()
        .try_into()
        .expect("failed to parse MessagePayload PDA data");
    assert_eq!(*parsed.payload_hash, expected_payload_hash);
}
