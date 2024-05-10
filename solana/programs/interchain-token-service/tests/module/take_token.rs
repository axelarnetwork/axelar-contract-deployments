use anyhow::{Ok, Result};
use interchain_token_service::get_interchain_token_service_associated_token_account;
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
async fn take_token_lock_unlock_success() -> Result<()> {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let destination = Keypair::new();
    let mint_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_transfer = 100;
    let gateway_operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&gateway_operators))
        .await;
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
            100,
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

    // Fund ITS ATA
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

    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount_to_transfer,
        "ITS ATA; incorect balance"
    );
    let account = fixture
        .banks_client
        .get_account(token_manager_root_pda_account_data.associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, 0,
        "Token Manager ATA; incorect balance"
    );

    // Action
    // Transfer from ITS ATA -> Token Manager ATA
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_take_token_lock_unlock_instruction(
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
    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_tokens_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, 0,
        "ITS ATA still has tokens; should be zero"
    );

    let account = fixture
        .banks_client
        .get_account(token_manager_root_pda_account_data.associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount_to_transfer,
        "Token Manager ATA; incorrect balance"
    );

    Ok(())
}

#[tokio::test]
async fn take_token_mint_burn_success() -> Result<()> {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let delegate_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint_preparations = 100;
    let amount_to_burn = 50;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&[]))
        .await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let mint_account_pda = fixture
        .init_new_mint(interchain_token_service_root_pda)
        .await;

    let (owner_of_its_ata_for_user_tokens_pda, _) =
        get_interchain_token_service_associated_token_account(
            &interchain_token_service_root_pda,
            &delegate_authority.pubkey(),
            &mint_account_pda,
            &interchain_token_service::id(),
        )?;
    let its_ata_for_user_pda = spl_associated_token_account::get_associated_token_address(
        &owner_of_its_ata_for_user_tokens_pda,
        &mint_account_pda,
    );

    // Fund as part of the preparations
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_mint_burn_instruction(
                amount_to_mint_preparations,
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_pda,
                &mint_account_pda,
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

    let account = fixture
        .banks_client
        .get_account(its_ata_for_user_pda)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();
    assert_eq!(
        token_account.amount, amount_to_mint_preparations,
        "incorrect amount after minting"
    );

    // Action
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_take_token_mint_burn_instruction(
                amount_to_burn,
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
                &owner_of_its_ata_for_user_tokens_pda,
                &its_ata_for_user_pda,
                &mint_account_pda,
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
        token_account.amount,
        amount_to_mint_preparations - amount_to_burn,
        "incorrect amount after burning"
    );

    Ok(())
}
