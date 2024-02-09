use anyhow::{Ok, Result};
use gateway::accounts::GatewayConfig;
use interchain_token_service::{
    get_interchain_token_service_associated_token_account, TokenManagerType,
};
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::Account;
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
