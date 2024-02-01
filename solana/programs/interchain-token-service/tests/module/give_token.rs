use anyhow::{Ok, Result};
use borsh::BorshDeserialize;
use gateway::accounts::GatewayConfig;
use gateway::types::PubkeyWrapper;
use interchain_token_service::{get_interchain_token_service_root_pda, TokenManagerType};
use solana_program::instruction::{AccountMeta, Instruction, InstructionError};
use solana_program::keccak::hash;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program_test::{processor, tokio, BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_token::state::{Account, Mint};

#[tokio::test]
async fn give_token_mint_burn_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;

    let rent = Rent::default();
    let amount: u64 = 100;

    // Mint Account / Owned by SPL Token Program
    let token_address = Keypair::new();
    let token_manager = Keypair::new();
    let to = Keypair::new();

    // Setup Root PDA / Initialize

    let gas_service_root_pda = fixture.init_gas_service().await;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda =
        get_interchain_token_service_root_pda(&gateway_root_pda, &gas_service_root_pda);

    let ix = interchain_token_service::instruction::build_initialize_instruction(
        &fixture.payer.pubkey(),
        &interchain_token_service_root_pda,
        &gateway_root_pda,
        &gas_service_root_pda,
    )
    .unwrap();
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
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
                &token_address.pubkey(),
                rent.minimum_balance(Mint::LEN),
                Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &token_address.pubkey(),
                &interchain_token_service_root_pda,
                None,
                0,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &token_address],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Setup Destination / To Account

    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &fixture.payer.pubkey(),
                &to.pubkey(),
                rent.minimum_balance(Account::LEN),
                Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &to.pubkey(),
                &token_address.pubkey(),
                &interchain_token_service_root_pda,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &to],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Final Part: GiveToken

    let ix = interchain_token_service::instruction::build_give_token_instruction(
        TokenManagerType::MintBurn,
        amount,
        token_address.pubkey(),
        token_manager.pubkey(),
        to.pubkey(),
        interchain_token_service_root_pda,
        &gateway_root_pda,
        &gas_service_root_pda,
    )?;

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[ix],
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
        .get_account(to.pubkey())
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(token_account.amount, amount);

    Ok(())
}
