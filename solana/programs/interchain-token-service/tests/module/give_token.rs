use anyhow::{Ok, Result};
use gateway::accounts::GatewayConfig;
use interchain_token_service::error::InterchainTokenServiceError;
use interchain_token_service::{
    get_interchain_token_service_associated_token_account, TokenManagerType,
};
use interchain_token_transfer_gmp::ethers_core::types::U256;
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use solana_program::instruction::InstructionError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account;
use test_fixtures::account::CheckValidPDAInTests;
use token_manager::get_token_manager_account;

#[tokio::test]
async fn give_token_mint_burn_ata_create_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;
    let token_manager = Keypair::new();
    let delegate_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint: u64 = 100;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;

    let mint_account = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (its_ata, _its_ata_bump) = get_interchain_token_service_associated_token_account(
        &interchain_token_service_root_pda,
        &delegate_authority.pubkey(),
        &mint_account,
        &interchain_token_service::id(),
    )?;

    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(&its_ata, &mint_account);

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_instruction(
                TokenManagerType::MintBurn,
                amount_to_mint,
                &fixture.payer.pubkey(),
                &mint_account,
                &token_manager.pubkey(),
                &delegate_authority.pubkey(),
                &associated_token_account,
                &interchain_token_service_root_pda,
                &gateway_root_pda,
                &gas_service_root_pda,
                &its_ata,
            )?,
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = fixture
        .banks_client
        .get_account(associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(token_account.amount, amount_to_mint);
    assert_eq!(
        token_account.delegate,
        solana_sdk::program_option::COption::Some(delegate_authority.pubkey())
    );
    assert_eq!(token_account.delegated_amount, amount_to_mint);
    assert_eq!(token_account.owner, its_ata);

    Ok(())
}

#[tokio::test]
async fn give_token_mint_ata_already_exist_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;
    let token_manager = Keypair::new();
    let delegate_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint: u64 = 100;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;

    let mint_account = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (its_ata, _its_ata_bump) = get_interchain_token_service_associated_token_account(
        &interchain_token_service_root_pda,
        &delegate_authority.pubkey(),
        &mint_account,
        &interchain_token_service::id(),
    )?;

    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(&its_ata, &mint_account);

    // Setup ATA to skip initialization within the program
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &fixture.payer.pubkey(),
                &its_ata,
                &mint_account,
                &spl_token::id(),
            ),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = fixture
        .banks_client
        .get_account(associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(token_account.owner, its_ata);
    assert_eq!(token_account.mint, mint_account);

    // Actual Transaction

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_instruction(
                TokenManagerType::MintBurn,
                amount_to_mint,
                &fixture.payer.pubkey(),
                &mint_account,
                &token_manager.pubkey(),
                &delegate_authority.pubkey(),
                &associated_token_account,
                &interchain_token_service_root_pda,
                &gateway_root_pda,
                &gas_service_root_pda,
                &its_ata,
            )?,
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = fixture
        .banks_client
        .get_account(associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(token_account.amount, amount_to_mint);
    assert_eq!(
        token_account.delegate,
        solana_sdk::program_option::COption::Some(delegate_authority.pubkey())
    );
    assert_eq!(token_account.delegated_amount, amount_to_mint);
    assert_eq!(token_account.owner, its_ata);

    Ok(())
}

#[tokio::test]
async fn give_token_lock_burn_non_ata_doesnt_exist_failure() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;
    let token_manager = Keypair::new();
    let destination = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint: u64 = 100;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;

    let mint_account = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (its_ata, _its_ata_bump) = get_interchain_token_service_associated_token_account(
        &interchain_token_service_root_pda,
        &destination.pubkey(),
        &mint_account,
        &interchain_token_service::id(),
    )?;

    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(&its_ata, &mint_account);

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_instruction(
                TokenManagerType::LockUnlock,
                amount_to_mint,
                &fixture.payer.pubkey(),
                &mint_account,
                &token_manager.pubkey(),
                &destination.pubkey(),
                &associated_token_account,
                &interchain_token_service_root_pda,
                &gateway_root_pda,
                &gas_service_root_pda,
                &its_ata,
            )?,
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    assert_eq!(
        fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(
            0,
            InstructionError::Custom(
                InterchainTokenServiceError::UninitializedAssociatedTokenAccount as u32
            )
        )
    );

    Ok(())
}

#[tokio::test]
async fn give_token_lock_burn_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;
    // let token_manager_ata = Keypair::new();
    let destination = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_transfer: u64 = 100;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;

    let mint_account = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (its_ata, _its_ata_bump) = get_interchain_token_service_associated_token_account(
        &interchain_token_service_root_pda,
        &destination.pubkey(),
        &mint_account,
        &interchain_token_service::id(),
    )?;

    let associated_token_account =
        spl_associated_token_account::get_associated_token_address(&its_ata, &mint_account);

    // Setup ATA to skip initialization within the program
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &fixture.payer.pubkey(),
                &its_ata,
                &mint_account,
                &spl_token::id(),
            ),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = fixture
        .banks_client
        .get_account(associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(token_account.owner, its_ata);
    assert_eq!(token_account.mint, mint_account);

    // Setup destination ATA token address
    let associated_token_account_destination =
        spl_associated_token_account::get_associated_token_address(
            &destination.pubkey(),
            &mint_account,
        );

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &fixture.payer.pubkey(),
                &destination.pubkey(),
                &mint_account,
                &spl_token::id(),
            ),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    let account = Account::unpack(
        &fixture
            .banks_client
            .get_account(associated_token_account_destination)
            .await
            .unwrap()
            .unwrap()
            .data,
    )
    .unwrap();

    assert_eq!(account.owner, destination.pubkey());
    assert_eq!(account.amount, 0);

    // let transaction: Transaction = Transaction::new_signed_with_payer(
    //     &[
    //         interchain_token_service::instruction::build_give_token_instruction(
    //             TokenManagerType::MintBurn,
    //             amount_to_mint,
    //             &fixture.payer.pubkey(),
    //             &mint_account,
    //             &token_manager.pubkey(),
    //             &destination.pubkey(),
    //             &associated_token_account,
    //             &interchain_token_service_root_pda,
    //             &gateway_root_pda,
    //             &gas_service_root_pda,
    //             &its_ata,
    //         )?,
    //     ],
    //     Some(&fixture.payer.pubkey()),
    //     &[&fixture.payer],
    //     recent_blockhash,
    // );
    // fixture
    //     .banks_client
    //     .process_transaction(transaction)
    //     .await
    //     .unwrap();

    ////////////////
    let token_id = Bytes32(keccak256("random-token-id"));
    let init_operator = Pubkey::from([0; 32]);

    let its_token_manager_permission_groups = fixture
        .derive_token_manager_permission_groups(
            &token_id,
            &interchain_token_service_root_pda,
            &init_operator,
        )
        .await;
    let token_manager_root_pda_pubkey = get_token_manager_account(
        &its_token_manager_permission_groups.operator_group.group_pda,
        &its_token_manager_permission_groups
            .flow_limiter_group
            .group_pda,
        &interchain_token_service_root_pda,
    );

    let ix = interchain_token_service::instruction::build_deploy_token_manager_instruction(
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
        &interchain_token_service_root_pda,
        &mint_account,
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

    let token_manager_ata =
        get_associated_token_address(&token_manager_root_pda_pubkey, &mint_account);

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
                &mint_account
            ),
            token_mint: mint_account,
        }
    );

    let account = fixture
        .banks_client
        .get_account(token_manager_ata)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(
        token_account.delegate,
        solana_sdk::program_option::COption::Some(interchain_token_service_root_pda)
    );

    ///////////////
    // let recent_blockhash = fixture.refresh_blockhash().await;
    // let transaction: Transaction = Transaction::new_signed_with_payer(
    //     &[
    //         interchain_token_service::instruction::build_give_token_instruction(
    //             TokenManagerType::LockUnlock,
    //             amount_to_transfer,
    //             &fixture.payer.pubkey(),
    //             &mint_account,
    //             &token_manager_ata,
    //             &associated_token_account_destination,
    //             &associated_token_account,
    //             &interchain_token_service_root_pda,
    //             &gateway_root_pda,
    //             &gas_service_root_pda,
    //             &its_ata,
    //         )?,
    //     ],
    //     Some(&fixture.payer.pubkey()),
    //     &[&fixture.payer],
    //     recent_blockhash,
    // );
    // fixture
    //     .banks_client
    //     .process_transaction(transaction)
    //     .await
    //     .unwrap();

    Ok(())
}
