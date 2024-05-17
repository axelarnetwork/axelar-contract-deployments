use axelar_message_primitives::command::DecodedCommand;
use axelar_message_primitives::EncodingScheme;
use gateway::state::GatewayApprovedCommand;
use interchain_token_service::instruction::from_external_chains::build_deploy_token_manager_from_gmp_instruction;
use interchain_token_transfer_gmp::ethers_core::types::U256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::axelar_message::custom_message;
use token_manager::TokenManagerType;

use crate::setup_its_root_fixture;

#[tokio::test]
async fn test_deploy_token_manager() {
    // Setup
    let (
        mut fixture,
        gas_service_root_pda,
        gateway_root_pda,
        interchain_token_service_root_pda,
        its_token_manager_permission_groups,
        token_manager_root_pda_pubkey,
        gateway_operators,
    ) = setup_its_root_fixture().await;
    let mint_authority = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let message_payload = build_deploy_token_manager_from_gmp_instruction(
        &interchain_token_service_root_pda,
        &gas_service_root_pda,
        &fixture.payer.pubkey(),
        &token_manager_root_pda_pubkey,
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .operator_group
            .group_pda_user_owner,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda_user_owner,
        &token_mint,
        DeployTokenManager {
            token_id: Bytes32(keccak256("random-token-id")),
            token_manager_type: U256::from(TokenManagerType::MintBurn as u8),
            params: vec![],
        },
        EncodingScheme::Borsh,
    );
    let message_to_execute =
        custom_message(interchain_token_service::id(), message_payload.clone()).unwrap();
    let (gateway_approved_message_pda, execute_data, _gateway_execute_data_pda) = fixture
        .fully_approve_messages(
            &gateway_root_pda,
            &[message_to_execute.clone()],
            &gateway_operators,
        )
        .await;
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pda[0])
        .await
        .expect("get_account")
        .expect("account not none");
    let DecodedCommand::ApproveMessages(command_to_execute) =
        execute_data.command_batch.commands[0].clone()
    else {
        panic!("Expected ApproveMessages command");
    };
    let data = GatewayApprovedCommand::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_command_approved(),
        "GatewayApprovedMessage should be approved"
    );

    // Action
    let ix = axelar_executable::construct_axelar_executable_ix(
        command_to_execute,
        message_payload.encode().unwrap(),
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

    // Assert
    // Operator group
    let op_group = fixture
        .banks_client
        .get_account(its_token_manager_permission_groups.operator_group.group_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = op_group
        .check_initialized_pda::<account_group::state::PermissionGroupAccount>(&account_group::id())
        .unwrap();

    // Operator account
    let operator = fixture
        .banks_client
        .get_account(
            its_token_manager_permission_groups
                .operator_group
                .group_pda_user,
        )
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = operator
        .check_initialized_pda::<account_group::state::PermissionAccount>(&account_group::id())
        .unwrap();
    // Flow limiter group
    let flow_group = fixture
        .banks_client
        .get_account(
            its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda,
        )
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = flow_group
        .check_initialized_pda::<account_group::state::PermissionGroupAccount>(&account_group::id())
        .unwrap();

    // Flow limiter account
    let flow_limiter = fixture
        .banks_client
        .get_account(
            its_token_manager_permission_groups
                .flow_limiter_group
                .group_pda_user,
        )
        .await
        .expect("get_account")
        .expect("account not none");
    let _ = flow_limiter
        .check_initialized_pda::<account_group::state::PermissionAccount>(&account_group::id())
        .unwrap();

    // Token manager account
    let token_manager_root_pda = fixture
        .banks_client
        .get_account(token_manager_root_pda_pubkey)
        .await
        .expect("get_account")
        .expect("account not none");
    let token_manager_root_pda =
        token_manager_root_pda
            .check_initialized_pda::<token_manager::state::TokenManagerRootAccount>(
                &token_manager::id(),
            )
            .unwrap();
    assert_eq!(
        token_manager_root_pda,
        token_manager::state::TokenManagerRootAccount {
            flow_limit: 0,
            associated_token_account: get_associated_token_address(
                &token_manager_root_pda_pubkey,
                &token_mint
            ),
            token_manager_type: token_manager::TokenManagerType::MintBurn,
            token_mint,
        }
    );
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pda[0])
        .await
        .expect("get_account")
        .expect("account not none");
    let data = GatewayApprovedCommand::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_validate_message_executed(),
        "GatewayApprovedMessage should be `executed`"
    );
}
