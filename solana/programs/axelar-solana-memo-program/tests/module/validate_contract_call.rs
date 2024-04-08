use axelar_executable::axelar_message_primitives::command::DecodedCommand;
use axelar_executable::axelar_message_primitives::{DestinationProgramId, EncodingScheme};
use axelar_solana_memo_program::from_axelar_to_solana::build_memo;
use gateway::state::GatewayApprovedCommand;
use itertools::Either;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
async fn test_successful_validate_contract_call_borsh_message() {
    test_successful_validate_contract_call(EncodingScheme::Borsh).await;
}

#[tokio::test]
async fn test_successful_validate_contract_call_abi_encoded_message() {
    test_successful_validate_contract_call(EncodingScheme::AbiEncoding).await;
}

async fn test_successful_validate_contract_call(encoding_scheme: EncodingScheme) {
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
        encoding_scheme,
    );
    let message_to_execute =
        custom_message(destination_program_id, message_payload.clone()).unwrap();
    let message_to_stall = custom_message(destination_program_id, message_payload.clone()).unwrap();
    let messages = [message_to_execute.clone(), message_to_stall.clone()]
        .into_iter()
        .map(Either::Left)
        .collect::<Vec<_>>();
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let (execute_data_pda, gatewa_execute_data, _raw_execute_data) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, weight_of_quorum)
        .await;
    let gateway_approved_message_pdas = fixture
        .init_pending_gateway_commands(
            &gateway_root_pda,
            &gatewa_execute_data.command_batch.commands,
        )
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_message_pdas,
        )
        .await;

    // Action: set message status as executed
    let DecodedCommand::ApproveContractCall(approved_message) =
        gatewa_execute_data.command_batch.commands[0].clone()
    else {
        panic!("expected ApproveContractCall command")
    };
    let ix = axelar_executable::construct_axelar_executable_ix(
        approved_message,
        message_payload.encode().unwrap(),
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
    let gateway_approved_command_data = gateway_approved_message
        .check_initialized_pda::<GatewayApprovedCommand>(&gateway::id())
        .unwrap();
    assert!(gateway_approved_command_data.is_command_executed());

    // The second message is still in Approved status
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pdas[1])
        .await
        .expect("get_account")
        .expect("account not none");
    let gateway_approved_command_data = gateway_approved_message
        .check_initialized_pda::<GatewayApprovedCommand>(&gateway::id())
        .unwrap();
    assert!(gateway_approved_command_data.is_contract_call_approved());

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
