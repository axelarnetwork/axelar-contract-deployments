use axelar_executable::axelar_message_primitives::DestinationProgramId;
use axelar_solana_memo_program::from_axelar_to_solana::build_memo;
use gateway::state::approved_message::MessageApprovalStatus;
use gateway::state::{GatewayApprovedMessage, GatewayConfig};
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
async fn test_successful_validate_contract_call() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let random_account_used_by_ix = Keypair::new();
    let weight_of_quorum = 14;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let destination_program_id = DestinationProgramId(axelar_solana_memo_program::id());
    let memo_string = "🐪🐪🐪🐪";
    let message_payload = build_memo(
        memo_string.as_bytes(),
        &[&random_account_used_by_ix.pubkey()],
    );
    let message_to_execute =
        custom_message(destination_program_id, message_payload.clone()).unwrap();
    let message_to_stall = custom_message(destination_program_id, message_payload.clone()).unwrap();
    let messages = vec![message_to_execute.clone(), message_to_stall.clone()];

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::new(
            0,
            fixture.init_operators_and_epochs(&operators),
        ))
        .await;
    let execute_data_pda = fixture
        .init_execute_data(&gateway_root_pda, &messages, operators, weight_of_quorum)
        .await;
    let gateway_approved_message_pdas = fixture
        .init_pending_gateway_messages(&gateway_root_pda, &messages)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_message_pdas,
        )
        .await;

    // Action: set message status as executed
    let ix = axelar_executable::construct_axelar_executable_ix(
        &message_to_execute,
        message_payload.encode(),
        gateway_approved_message_pdas[0],
        gateway_root_pda,
    )
    .unwrap();
    let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    let tx = fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert!(tx.result.is_ok(), "transaction failed");

    // Assert
    // First message should be executed
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pdas[0])
        .await
        .expect("get_account")
        .expect("account not none");
    let gateway_approved_message_data = gateway_approved_message
        .check_initialized_pda::<GatewayApprovedMessage>(&gateway::id())
        .unwrap();
    assert_eq!(
        gateway_approved_message_data.status,
        MessageApprovalStatus::Executed
    );

    // The second message is still in Approved status
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pdas[1])
        .await
        .expect("get_account")
        .expect("account not none");
    let gateway_approved_message_data = gateway_approved_message
        .check_initialized_pda::<GatewayApprovedMessage>(&gateway::id())
        .unwrap();
    assert_eq!(
        gateway_approved_message_data.status,
        MessageApprovalStatus::Approved
    );

    // We can get the memo from the logs
    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains(&format!(
            "{:?}-{}-{}",
            random_account_used_by_ix.pubkey(),
            false,
            false
        ))),
        "expected memo not found in logs"
    );
    assert!(
        !log_msgs
            .iter()
            .filter(|log| !log.as_str().contains("Provided account",))
            // Besides the initial provided account, there are NO other provided accounts
            .any(|log| log.as_str().contains("Provided account")),
        "There was an unexpected provided account (appended to the logs)"
    );
}
