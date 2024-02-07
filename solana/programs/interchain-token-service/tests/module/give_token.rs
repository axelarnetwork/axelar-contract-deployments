use anyhow::{Ok, Result};
use gateway::accounts::GatewayConfig;
use interchain_token_service::{
    get_interchain_token_service_associated_token_account, get_interchain_token_service_root_pda,
    TokenManagerType,
};
use solana_program::program_pack::Pack;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_token::state::{Account, Mint};

#[tokio::test]
async fn give_token_mint_ata_create_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;

    let rent = Rent::default();
    let amount: u64 = 100;

    // aka "token address"
    let mint_account = Keypair::new();
    let token_manager = Keypair::new();
    // aka wallet address / "to"
    let delegate_authority = Keypair::new();

    let gas_service_root_pda = fixture.init_gas_service().await;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let interchain_token_service_root_pda =
        get_interchain_token_service_root_pda(&gateway_root_pda, &gas_service_root_pda);

    let (its_ata, _its_ata_bump) = get_interchain_token_service_associated_token_account(
        &interchain_token_service_root_pda,
        &delegate_authority.pubkey(),
        &mint_account.pubkey(),
        &interchain_token_service::id(),
    )?;

    let associated_token_account = spl_associated_token_account::get_associated_token_address(
        &its_ata,
        &mint_account.pubkey(),
    );

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_initialize_instruction(
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
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

    // Setup Mint Account

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &fixture.payer.pubkey(),
                &mint_account.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_account.pubkey(),
                &interchain_token_service_root_pda,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &mint_account],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Final Part: GiveToken

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_instruction(
                TokenManagerType::MintBurn,
                amount,
                fixture.payer.pubkey(),
                mint_account.pubkey(),
                token_manager.pubkey(),
                delegate_authority.pubkey(),
                associated_token_account,
                interchain_token_service_root_pda,
                gateway_root_pda,
                gas_service_root_pda,
                its_ata,
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

    assert_eq!(token_account.amount, amount);
    assert_eq!(
        token_account.delegate,
        solana_sdk::program_option::COption::Some(delegate_authority.pubkey())
    );
    assert_eq!(token_account.delegated_amount, amount);
    assert_eq!(token_account.owner, its_ata);

    Ok(())
}

#[tokio::test]
async fn give_token_mint_ata_already_exist_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;

    let rent = Rent::default();
    let amount: u64 = 100;

    // aka "token address"
    let mint_account = Keypair::new();
    let token_manager = Keypair::new();
    // aka wallet address / "to"
    let delegate_authority = Keypair::new();

    let gas_service_root_pda = fixture.init_gas_service().await;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;
    let interchain_token_service_root_pda =
        get_interchain_token_service_root_pda(&gateway_root_pda, &gas_service_root_pda);

    let (its_ata, _its_ata_bump) = get_interchain_token_service_associated_token_account(
        &interchain_token_service_root_pda,
        &delegate_authority.pubkey(),
        &mint_account.pubkey(),
        &interchain_token_service::id(),
    )?;

    let associated_token_account = spl_associated_token_account::get_associated_token_address(
        &its_ata,
        &mint_account.pubkey(),
    );

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_initialize_instruction(
                &fixture.payer.pubkey(),
                &interchain_token_service_root_pda,
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

    // Setup Mint Account

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &fixture.payer.pubkey(),
                &mint_account.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_account.pubkey(),
                &interchain_token_service_root_pda,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &mint_account],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Setup ATA
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            spl_associated_token_account::instruction::create_associated_token_account(
                &fixture.payer.pubkey(),
                &its_ata,
                &mint_account.pubkey(),
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
    assert_eq!(token_account.mint, mint_account.pubkey());

    // Final Part: GiveToken

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_give_token_instruction(
                TokenManagerType::MintBurn,
                amount,
                fixture.payer.pubkey(),
                mint_account.pubkey(),
                token_manager.pubkey(),
                delegate_authority.pubkey(),
                associated_token_account,
                interchain_token_service_root_pda,
                gateway_root_pda,
                gas_service_root_pda,
                its_ata,
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

    assert_eq!(token_account.amount, amount);
    assert_eq!(
        token_account.delegate,
        solana_sdk::program_option::COption::Some(delegate_authority.pubkey())
    );
    assert_eq!(token_account.delegated_amount, amount);
    assert_eq!(token_account.owner, its_ata);

    Ok(())
}
