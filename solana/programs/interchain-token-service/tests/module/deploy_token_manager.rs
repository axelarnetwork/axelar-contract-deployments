use gateway::accounts::GatewayConfig;
use interchain_token_transfer_gmp::ethers_core::types::U256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use token_manager::get_token_manager_account;

#[tokio::test]
async fn test_deploy_token_manager() {
    // Setup
    let mut fixture = super::utils::TestFixture::new().await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Pubkey::from([0; 32]);

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let its_token_manager_permission_groups = fixture
        .derive_token_manager_permission_groups(
            &token_id,
            &interchain_token_service_root_pda,
            &init_operator,
        )
        .await;
    let token_manager_root_pda = get_token_manager_account(
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &interchain_token_service_root_pda,
    );

    // Action
    let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
        &fixture.payer.pubkey(),
        &token_manager_root_pda,
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .operator_group
            .group_pda_user,
        &its_token_manager_permission_groups
            .operator_group
            .group_pda_user_owner,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda_user,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda_user_owner,
        &interchain_token_service_root_pda,
        DeployTokenManager {
            token_id: Bytes32(keccak256("random-token-id")),
            token_manager_type: U256::from(42),
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
        .check_initialized_pda::<interchain_token_service::state::RootPDA>(&account_group::id())
        .unwrap();

    // Token manager account
    let token_manager_root_pda = fixture
        .banks_client
        .get_account(token_manager_root_pda)
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
        token_manager::state::TokenManagerRootAccount { flow_limit: 0 }
    )
}
