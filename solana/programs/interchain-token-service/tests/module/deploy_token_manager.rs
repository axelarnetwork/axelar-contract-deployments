use std::ops::Add;

use gateway::accounts::GatewayApprovedMessage;
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
    let deploy_token_manager_messages = [(
        test_fixtures::axelar_message::message().unwrap(),
        interchain_token_service_root_pda,
    )];
    let gateway_approved_message_pda = fixture
        .fully_approve_messages(
            &gateway_root_pda,
            &deploy_token_manager_messages,
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
    let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
        &gateway_approved_message_pda,
        &gateway_root_pda,
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
            token_manager_type: U256::from(token_manager::TokenManagerType::MintBurn as u8),
            params: vec![],
        },
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
        .get_account(gateway_approved_message_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = GatewayApprovedMessage::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_executed(),
        "GatewayApprovedMessage should be `executed`"
    );
}

#[tokio::test]
#[should_panic(expected = "TransactionError(InstructionError(0, Custom(34)))")]
async fn test_deploy_token_manager_failed_when_message_not_approved() {
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
    let messages = [(
        test_fixtures::axelar_message::message().unwrap(),
        interchain_token_service_root_pda,
    )];
    let weight_of_quorum = gateway_operators
        .iter()
        .fold(cosmwasm_std::Uint256::zero(), |acc, i| acc.add(i.weight));
    let weight_of_quorum = U256::from_big_endian(&weight_of_quorum.to_be_bytes());
    let _execute_data_pda = fixture
        .init_execute_data(
            &gateway_root_pda,
            &messages
                .iter()
                .map(|(message, _executer)| message.clone())
                .collect::<Vec<_>>(),
            gateway_operators,
            weight_of_quorum.as_u128(),
        )
        .await;
    let gateway_approved_message_pda = fixture
        .init_pending_gateway_messages(&gateway_root_pda, &messages)
        .await[0];
    let gateway_approved_message = fixture
        .banks_client
        .get_account(gateway_approved_message_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = GatewayApprovedMessage::unpack_from_slice(gateway_approved_message.data()).unwrap();
    assert!(
        data.is_pending(),
        "GatewayApprovedMessage should be approved"
    );
    // NOTE: we don't `gateway.execute` (thus approve) the message

    // Action - errors out
    let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
        &gateway_approved_message_pda,
        &gateway_root_pda,
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
            token_manager_type: U256::from(token_manager::TokenManagerType::MintBurn as u8),
            params: vec![],
        },
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

#[tokio::test]
#[should_panic(expected = "TransactionError(InstructionError(0, Custom(34)))")]
async fn test_deploy_token_manager_cannot_execute_message_twice() {
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
    let deploy_token_manager_messages = [(
        test_fixtures::axelar_message::message().unwrap(),
        interchain_token_service_root_pda,
    )];
    let gateway_approved_message_pda = fixture
        .fully_approve_messages(
            &gateway_root_pda,
            &deploy_token_manager_messages,
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
    let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
        &gateway_approved_message_pda,
        &gateway_root_pda,
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
            token_manager_type: U256::from(token_manager::TokenManagerType::MintBurn as u8),
            params: vec![],
        },
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone()],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone()],
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
