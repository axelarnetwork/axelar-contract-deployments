use anyhow::{Ok, Result};
use gateway::accounts::GatewayConfig;
use interchain_token_service::get_interchain_token_service_associated_token_account;
use solana_program::program_option::COption;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::Account;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;
use token_manager::TokenManagerType;

use crate::program_test;

#[tokio::test]
async fn give_token_lock_unlock_success() -> Result<()> {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let destination = Keypair::new();
    let mint_authority = Keypair::new();
    let amount_to_transfer = 100;
    let gateway_operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::new(
            0,
            fixture.init_operators_and_epochs(&gateway_operators),
        ))
        .await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let mint_account_pda = fixture.init_new_mint(mint_authority.pubkey()).await;
    let (
        _token_manager_root_pda,
        token_manager_root_pda_account_data,
        _token_manager_handler_groups,
    ) = fixture
        .init_new_token_manager(
            interchain_token_service_root_pda,
            gas_service_root_pda,
            mint_account_pda,
            gateway_root_pda,
            TokenManagerType::LockUnlock,
            gateway_operators,
        )
        .await;
    fixture
        .mint_tokens_to(
            mint_account_pda,
            token_manager_root_pda_account_data.associated_token_account,
            mint_authority.insecure_clone(),
            200,
        )
        .await;
    let (owner_of_its_ata_for_user_tokens_pda, _its_ata_bump) =
        get_interchain_token_service_associated_token_account(
            &interchain_token_service_root_pda,
            &destination.pubkey(),
            &mint_account_pda,
            &interchain_token_service::id(),
        )?;
    let its_ata_for_user_tokens_pda = spl_associated_token_account::get_associated_token_address(
        &owner_of_its_ata_for_user_tokens_pda,
        &mint_account_pda,
    );

    // Action
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_lock_unlock_instruction(
                amount_to_transfer,
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &token_manager_root_pda_account_data.associated_token_account,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_tokens_pda,
                &mint_account_pda,
                &destination.pubkey(),
                &gateway_root_pda,
                &gas_service_root_pda,
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

    // Assert
    // get balance of destination wallet
    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount_to_transfer,
        "ITS ATA for user wallet did not match"
    );

    Ok(())
}

#[tokio::test]
async fn give_token_mint_burn_success() -> Result<()> {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let delegate_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint = 100;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let mint_account = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (owner_of_its_ata_for_user_tokens_pda, _) =
        get_interchain_token_service_associated_token_account(
            &interchain_token_service_root_pda,
            &delegate_authority.pubkey(),
            &mint_account,
            &interchain_token_service::id(),
        )?;
    let its_ata_for_user_pda = spl_associated_token_account::get_associated_token_address(
        &owner_of_its_ata_for_user_tokens_pda,
        &mint_account,
    );

    // Action
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_mint_burn_instruction(
                amount_to_mint,
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_pda,
                &mint_account,
                &delegate_authority.pubkey(),
                &gateway_root_pda,
                &gas_service_root_pda,
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

    // Assert
    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount_to_mint,
        "ITS ATA for user wallet did not match"
    );
    assert_eq!(
        token_account.owner, owner_of_its_ata_for_user_tokens_pda,
        "ITS ATA for user wallet owner did not match"
    );
    assert_eq!(
        token_account.delegate,
        COption::Some(delegate_authority.pubkey()),
        "Delegate did not match"
    );

    Ok(())
}
