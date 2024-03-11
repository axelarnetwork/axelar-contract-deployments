use gateway::state::GatewayApprovedMessage;
use interchain_token_transfer_gmp::ethers_core::types::U256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, InterchainTransfer};
use solana_program_test::tokio;
use solana_sdk::account::ReadableAccount;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::axelar_message::custom_message;

use crate::setup_its_root_fixture;

#[tokio::test]
#[should_panic(expected = "TransactionError(InstructionError(0, ProgramFailedToComplete))")]
async fn test_interchain_transfer() {
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
    let message_payload = interchain_token_service::instruction::from_external_chains::build_interchain_transfer_from_gmp_instruction(
        &interchain_token_service_root_pda,
        &gas_service_root_pda,
        InterchainTransfer { token_id: Bytes32(keccak256("random-token-id")), source_address: Pubkey::new_unique().to_bytes().to_vec(), destination_address: Pubkey::new_unique().to_bytes().to_vec(), amount: U256::from(100), data: Vec::new() },
        );
    let message_to_execute =
        custom_message(interchain_token_service::id(), message_payload.clone()).unwrap();
    let gateway_approved_message_pda = fixture
        .fully_approve_messages(
            &gateway_root_pda,
            &[message_to_execute.clone()],
            gateway_operators,
        )
        .await[0];
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = GatewayApprovedMessage::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_approved(),
        "GatewayApprovedMessage should be approved"
    );

    // Action
    let ix = axelar_executable::construct_axelar_executable_ix(
        &message_to_execute,
        message_payload.encode(),
        gateway_approved_message_pda,
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
