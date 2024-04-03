use axelar_message_primitives::command::DecodedCommand;
use gateway::state::GatewayApprovedCommand;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployInterchainToken};
use solana_program_test::tokio;
use solana_sdk::account::ReadableAccount;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::Either;

use crate::setup_its_root_fixture;

#[tokio::test]
#[should_panic(expected = "TransactionError(InstructionError(0, ProgramFailedToComplete))")]
async fn test_deploy_interchain_token() {
    // Setup
    let (
        mut fixture,
        gas_service_root_pda,
        gateway_root_pda,
        interchain_token_service_root_pda,
        _its_token_manager_permission_groups,
        _token_manager_root_pda_pubkey,
        gateway_operators,
    ) = setup_its_root_fixture().await;
    let message_payload = interchain_token_service::instruction::from_external_chains::build_deploy_interchain_token_from_gmp_instruction(
        &interchain_token_service_root_pda,
        &gas_service_root_pda,
        DeployInterchainToken { token_id: Bytes32(keccak256("random-token-id")), name: "EgierToken".to_string(), symbol: "EGR".to_string(), decimals: 18, minter: Pubkey::new_unique().to_bytes().to_vec() },
        );
    let message_to_execute =
        custom_message(interchain_token_service::id(), message_payload.clone()).unwrap();
    let (gateway_approved_message_pda, execute_data, _gateway_execute_data_pda) = fixture
        .fully_approve_messages(
            &gateway_root_pda,
            &[Either::Left(message_to_execute.clone())],
            gateway_operators,
        )
        .await;
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pda[0])
        .await
        .expect("get_account")
        .expect("account not none");
    let DecodedCommand::ApproveContractCall(command_to_execute) =
        execute_data.command_batch.commands[0].clone()
    else {
        panic!("Expected ApproveContractCall command");
    };
    let data = GatewayApprovedCommand::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_contract_call_approved(),
        "GatewayApprovedMessage should be approved"
    );

    // Action
    let ix = axelar_executable::construct_axelar_executable_ix(
        command_to_execute,
        message_payload.encode(),
        gateway_approved_message_pda[0],
        gateway_root_pda,
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}
