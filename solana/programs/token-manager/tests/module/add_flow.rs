use solana_program::clock::Clock;
use solana_program::program_pack::Pack;
use solana_program_test::{tokio, BanksClientError};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use test_fixtures::test_setup::interchain_token_transfer_gmp::Bytes32;
use test_fixtures::test_setup::TestFixture;
use token_manager::instruction::FlowToAdd;
use token_manager::{get_token_flow_account, CalculatedEpoch};

use crate::program_test;

#[tokio::test]
async fn test_add_flow_success() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
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
            500,
            gateway_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;

    // Action
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda_pubkey,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 90,
            add_flow_out: 5,
        },
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
    let token_flow_pda = fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(token_flow_pda.owner, token_manager::id());

    let data = token_flow_pda
        .check_initialized_pda::<token_manager::state::TokenManagerFlowInOutAccount>(
            &token_manager::ID,
        )
        .unwrap();
    assert_eq!(
        data,
        token_manager::state::TokenManagerFlowInOutAccount {
            flow_in: 90,
            flow_out: 5,
        }
    );
}

#[tokio::test]
async fn test_add_flow_2_times_success() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
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
            500,
            gateway_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda_pubkey,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );

    // Action
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 90,
            add_flow_out: 5,
        },
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone(), ix],
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
    let token_flow_pda = fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(token_flow_pda.owner, token_manager::id());
    assert_eq!(
        token_flow_pda.data.len(),
        token_manager::state::TokenManagerFlowInOutAccount::LEN
    );
    let data = token_flow_pda
        .check_initialized_pda::<token_manager::state::TokenManagerFlowInOutAccount>(
            &token_manager::ID,
        )
        .unwrap();
    assert_eq!(
        data,
        token_manager::state::TokenManagerFlowInOutAccount {
            flow_in: 180,
            flow_out: 10,
        }
    );
}

#[tokio::test]
async fn test_add_flow_old_pdas_failure() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
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
            500,
            gateway_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;
    let block_timestamp = 10; // super old timestamp
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda_pubkey,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );

    // Action
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 90,
            add_flow_out: 5,
        },
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    let res = fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    // Assert
    assert!(matches!(res, BanksClientError::TransactionError(_)));
    assert!(fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .is_none());
}

#[tokio::test]
async fn test_add_flow_in_exceeds_limit() {
    // Setup
    let flow_limit = 500;
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
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
            gateway_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda_pubkey,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: flow_limit - 1,
            add_flow_out: 0,
        },
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

    // Action
    let ix2 = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: flow_limit + 1,
            add_flow_out: 0,
        },
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    // Assert
    let token_flow_pda = fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let data = token_flow_pda
        .check_initialized_pda::<token_manager::state::TokenManagerFlowInOutAccount>(
            &token_manager::ID,
        )
        .unwrap();
    assert_eq!(
        data,
        token_manager::state::TokenManagerFlowInOutAccount {
            flow_in: flow_limit - 1,
            flow_out: 0,
        }
    );
}

#[tokio::test]
async fn test_add_flow_in_success() {
    // Setup
    let flow_limit = 5;
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
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
            gateway_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda_pubkey,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 1,
            add_flow_out: 0,
        },
    )
    .unwrap();

    // Action
    for idx in 0..flow_limit {
        let transaction = Transaction::new_signed_with_payer(
            &[ix.clone()],
            Some(&fixture.payer.pubkey()),
            &[&fixture.payer, &init_flow_limiter],
            fixture.banks_client.get_latest_blockhash().await.unwrap(),
        );
        fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // sleep for 500ms
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let token_flow_pda = fixture
            .banks_client
            .get_account(token_flow_pda)
            .await
            .expect("get_account")
            .expect("account not none");

        let data = token_flow_pda
            .check_initialized_pda::<token_manager::state::TokenManagerFlowInOutAccount>(
                &token_manager::ID,
            )
            .unwrap();
        assert_eq!(
            data,
            token_manager::state::TokenManagerFlowInOutAccount {
                flow_in: idx + 1,
                flow_out: 0,
            }
        );
    }

    // Assert
    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone()],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
}

#[tokio::test]
async fn test_add_flow_out_success() {
    // Setup
    let flow_limit = 5;
    let mut fixture = TestFixture::new(program_test()).await;
    let mint_authority = Keypair::new();
    let interchain_token_service_root_pda = Keypair::new();
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Keypair::new();
    let init_flow_limiter = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
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
            gateway_config_pda,
            token_mint,
            interchain_token_service_root_pda.pubkey(),
        )
        .await;
    let block_timestamp = fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap()
        .unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda_pubkey,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &token_flow_pda,
        &groups.flow_limiter_group.group_pda,
        &groups.flow_limiter_group.group_pda_user,
        &groups.flow_limiter_group.group_pda_user_owner,
        &groups.operator_group.group_pda,
        &interchain_token_service_root_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 0,
            add_flow_out: 1,
        },
    )
    .unwrap();

    // Action
    for idx in 0..flow_limit {
        let transaction = Transaction::new_signed_with_payer(
            &[ix.clone()],
            Some(&fixture.payer.pubkey()),
            &[&fixture.payer, &init_flow_limiter],
            fixture.banks_client.get_latest_blockhash().await.unwrap(),
        );
        fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // sleep for 500ms
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let token_flow_pda = fixture
            .banks_client
            .get_account(token_flow_pda)
            .await
            .expect("get_account")
            .expect("account not none");

        let data = token_flow_pda
            .check_initialized_pda::<token_manager::state::TokenManagerFlowInOutAccount>(
                &token_manager::ID,
            )
            .unwrap();
        assert_eq!(
            data,
            token_manager::state::TokenManagerFlowInOutAccount {
                flow_out: idx + 1,
                flow_in: 0,
            }
        );
    }

    // Assert
    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone()],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &init_flow_limiter],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
}
