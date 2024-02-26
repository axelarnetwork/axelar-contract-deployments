use gmp_gateway::accounts::approved_message::MessageApprovalStatus;
use gmp_gateway::accounts::{GatewayApprovedMessage, GatewayConfig};
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::axelar_message::message;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
async fn test_successful_validate_contract_call() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let allowed_executer = Keypair::new();
    let weight_of_quorum = 14;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let messages = vec![
        (message().unwrap(), allowed_executer.pubkey()),
        (message().unwrap(), allowed_executer.pubkey()),
    ];

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::new(
            0,
            fixture.init_operators_and_epochs(&operators),
        ))
        .await;
    let execute_data_pda = fixture
        .init_execute_data(
            &gateway_root_pda,
            &messages
                .iter()
                .map(|(message, _executer)| message.clone())
                .collect::<Vec<_>>(),
            operators,
            weight_of_quorum,
        )
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
    let ix = gmp_gateway::instructions::validate_contract_call(
        &fixture.payer.pubkey(),
        &gateway_root_pda,
        &gateway_approved_message_pdas[0],
        &allowed_executer.pubkey(),
    )
    .unwrap();
    let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &allowed_executer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Assert
    // First message should be executed
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pdas[0])
        .await
        .expect("get_account")
        .expect("account not none");
    let gateway_approved_message_data = gateway_approved_message
        .check_initialized_pda::<GatewayApprovedMessage>(&gmp_gateway::id())
        .unwrap();
    assert_eq!(
        gateway_approved_message_data.status,
        MessageApprovalStatus::Executed
    );
    assert_eq!(
        gateway_approved_message_data.allowed_executer,
        allowed_executer.pubkey()
    );

    // The second message is still in Approved status
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pdas[1])
        .await
        .expect("get_account")
        .expect("account not none");
    let gateway_approved_message_data = gateway_approved_message
        .check_initialized_pda::<GatewayApprovedMessage>(&gmp_gateway::id())
        .unwrap();
    assert_eq!(
        gateway_approved_message_data.status,
        MessageApprovalStatus::Approved
    );
    assert_eq!(
        gateway_approved_message_data.allowed_executer,
        allowed_executer.pubkey()
    );
}
