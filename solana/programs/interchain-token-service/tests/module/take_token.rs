use anyhow::{Ok, Result};
use gateway::accounts::GatewayConfig;
use interchain_token_service::get_interchain_token_service_associated_token_account;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::Account;

#[tokio::test]
async fn take_token_mint_burn_success() -> Result<()> {
    // Setup
    let mut fixture = super::utils::TestFixture::new().await;
    let delegate_authority = Keypair::new();
    let gas_service_root_pda = fixture.init_gas_service().await;
    let amount_to_mint_preparations = 100;
    let amount_to_burn = 50;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
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
