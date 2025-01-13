use crate::initialize_message_payload::{
    get_message_account, initialize_message_payload_pda, message_to_command_id,
};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn successfully_close_message_payload_pda() {
    // Setup
    let mut runner = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42, 42])
        .build()
        .setup()
        .await;
    let message = random_message();
    // Allocate a relatively large amount of space so we get meaningful rent value for
    // asserting differences later on this test.
    let payload_size = 128;
    initialize_message_payload_pda(&mut runner, &message, payload_size).await;

    let previous_payer_account_balance = get_payer_account_balance(&mut runner).await;
    let previous_message_account_balance =
        get_message_payload_account_balance(&mut runner, &message).await;
    assert!(previous_message_account_balance > 0);

    // Build an instruction to close the message payload account
    let command_id = message_to_command_id(&message);
    let ix = axelar_solana_gateway::instructions::close_message_payload(
        runner.gateway_root_pda,
        runner.payer.pubkey(),
        command_id,
    )
    .unwrap();
    let tx = runner.send_tx(&[ix]).await.unwrap();
    assert!(tx.result.is_ok());
    // Assert that the message payload account state changed according to expected
    assert!(get_message_account(&mut runner, &message).await.is_none());

    // Assert that the payer account balance reclaimed the lamports from the message payload account
    let current_payer_account_balance = get_payer_account_balance(&mut runner).await;
    let expected_final_payer_balance = previous_payer_account_balance // what it had before
            + previous_message_account_balance; // what should've been reclaimed

    // This assertion is an approximation because of transaction fees.
    assert_close_enough(
        current_payer_account_balance,
        expected_final_payer_balance,
        1, // % tolerance
    );
}

async fn get_payer_account_balance(runner: &mut SolanaAxelarIntegrationMetadata) -> u64 {
    let payer_pubkey = runner.payer.pubkey();
    let payer_account = runner
        .get_account(&payer_pubkey, &solana_program::system_program::ID)
        .await;
    payer_account.lamports
}

async fn get_message_payload_account_balance(
    runner: &mut SolanaAxelarIntegrationMetadata,
    message: &Message,
) -> u64 {
    let message_payload_account = get_message_account(runner, message)
        .await
        .expect("error getting account");
    message_payload_account.lamports
}

/// Checks if a given `value` is within `tolerance_percent` % of the `target`.
#[allow(clippy::integer_division)]
#[allow(clippy::integer_division_remainder_used)]
#[allow(clippy::arithmetic_side_effects)]
fn assert_close_enough(value: u64, target: u64, tolerance_percent: u64) {
    assert!(tolerance_percent <= 100);
    assert!(value <= target);
    let tolerance = target * tolerance_percent / 100;

    assert!(
        (target - value) <= tolerance,
        "Value {value} is not within {tolerance}% of the target {target}.",
    );
}

#[test]
fn test_helper_fn() {
    assert_close_enough(99, 100, 1);
    assert_close_enough(99, 100, 2);
    assert_close_enough(80, 100, 20);
}
#[test]
#[should_panic(expected = "Value 98 is not within 1% of the target 100.")]
fn test_helper_fn_panics() {
    assert_close_enough(98, 100, 1);
}
