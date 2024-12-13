use crate::initialize_message_payload::{
    get_message_account, initialize_message_payload_pda, message_to_command_id,
};
use axelar_solana_gateway::state::message_payload::MessagePayload;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use axelar_solana_gateway_test_fixtures::SolanaAxelarIntegration;
use solana_program_test::tokio;

use solana_sdk::signer::Signer;

#[tokio::test]
async fn successfully_write_message_payload_pda() {
    // Setup
    let mut runner = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let message = random_message();
    let payload_size = 128u64;
    initialize_message_payload_pda(&mut runner, &message, payload_size).await;

    // Build an instruction to write ones to the first half of the buffer
    let command_id = message_to_command_id(&message);
    let all_ones = [1u8; 64];

    let ix = axelar_solana_gateway::instructions::write_message_payload(
        runner.gateway_root_pda,
        runner.payer.pubkey(),
        command_id,
        &all_ones,
        0,
    )
    .unwrap();
    let tx = runner.send_tx(&[ix]).await.unwrap();
    assert!(tx.result.is_ok());

    // Assert that the message payload account state changed according to expected
    let mut message_payload_account = get_message_account(&mut runner, &message)
        .await
        .expect("error getting account");

    let message_payload =
        MessagePayload::from_borrowed_account_data(&mut message_payload_account.data).unwrap();

    let (first_half, last_half) = message_payload.raw_payload.split_at(64);
    assert!(first_half.iter().all(|&x| x == 1));
    assert!(last_half.iter().all(|&x| x == 0));
}
