use gateway::state::GatewayConfig;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use test_fixtures::test_setup::interchain_token_transfer_gmp::Bytes32;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
async fn test_set_flow_limit() {
    // Setup
    const NEW_FLOW_LIMIT: u64 = 100;
    let flow_limit = 500;
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_root_config_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let groups = fixture
        .derive_token_manager_permission_groups(
            &token_id,
            &interchain_token_service_root_pda.pubkey(),
            &init_flow_limiter.pubkey(),
            &init_operator.pubkey(),
        )
        .await;
    fixture
        .setup_permission_group(&groups.flow_limiter_group)
        .await;
    fixture.setup_permission_group(&groups.operator_group).await;
    let token_manager_pda_pubkey = fixture
        .setup_token_manager(
            token_manager::TokenManagerType::LockUnlock,
            groups.clone(),
            flow_limit,
            gateway_root_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;

    // Action
    let ix = token_manager::instruction::build_set_flow_limit_instruction(
        &token_manager_pda_pubkey,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        NEW_FLOW_LIMIT,
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Assert
    let token_manager_pda = fixture
        .banks_client
        .get_account(token_manager_pda_pubkey)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = token_manager_pda
        .check_initialized_pda::<token_manager::state::TokenManagerRootAccount>(&token_manager::ID)
        .unwrap();
    assert_eq!(
        data,
        token_manager::state::TokenManagerRootAccount {
            flow_limit: NEW_FLOW_LIMIT,
            associated_token_account: get_associated_token_address(
                &token_manager_pda_pubkey,
                &token_mint
            ),
            token_mint,
            token_manager_type: token_manager::TokenManagerType::LockUnlock,
        }
    );
}
