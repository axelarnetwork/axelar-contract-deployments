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
use solana_program::{system_instruction, system_program};
use solana_program_test::{processor, tokio, BanksTransactionResultWithMetadata, ProgramTest};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::{Transaction, TransactionError};
use spl_associated_token_account;
use spl_token::state::{Account, Mint};

#[tokio::test]
async fn give_token_mint_ata_create_success() -> Result<()> {
    let mut fixture = super::utils::TestFixture::new().await;

    let rent = Rent::default();
    let amount: u64 = 100;

    // Aka "token address"
    let mint_account = Keypair::new();
    let token_manager = Keypair::new();
    // Aka previously "to"
    let wallet_address = Keypair::new();

    // Setup Root PDA / Initialize

    let gas_service_root_pda = fixture.init_gas_service().await;

    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::default())
        .await;

    let interchain_token_service_root_pda =
        get_interchain_token_service_root_pda(&gateway_root_pda, &gas_service_root_pda);

    // TODO: separate function?
    let (the_pda_derived, the_pda_bump) = Pubkey::find_program_address(
        &[
            &interchain_token_service_root_pda.to_bytes(),
            &wallet_address.pubkey().to_bytes(),
            &mint_account.pubkey().to_bytes(),
        ],
        &interchain_token_service::id(),
    );

    // Derive ATA / new "to" address
    let associated_token_account = spl_associated_token_account::get_associated_token_address(
        &the_pda_derived,
        &mint_account.pubkey(),
    );

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

    // // // Final Part: GiveToken

    let ix = interchain_token_service::instruction::build_give_token_instruction(
        TokenManagerType::MintBurn,
        amount,
        fixture.payer.pubkey(),
        mint_account.pubkey(),
        token_manager.pubkey(),
        wallet_address.pubkey(),  // owner of "to" account
        associated_token_account, // previously "to" / ATA
        interchain_token_service_root_pda,
        gateway_root_pda,
        gas_service_root_pda,
        the_pda_derived,
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
        .get_account(associated_token_account)
        .await
        .unwrap()
        .unwrap();
    let token_account = Account::unpack(&account.data).unwrap();

    assert_eq!(token_account.amount, amount);
    // assert_ne!(token_account.amount, amount + 1);

    Ok(())
}

// #[tokio::test]
// async fn give_token_mint_ata_already_exist_success() -> Result<()> {
//     let mut fixture = super::utils::TestFixture::new().await;

//     let rent = Rent::default();
//     let amount: u64 = 100;

//     // Aka "token address"
//     let mint_account = Keypair::new();
//     let token_manager = Keypair::new();
//     // Aka previously "to"
//     let wallet_address = Keypair::new();

//     // Derive ATA / new "to" address
//     let associated_token_account =
// spl_associated_token_account::get_associated_token_address(
//         &wallet_address.pubkey(),
//         &mint_account.pubkey(),
//     );

//     // Setup Root PDA / Initialize

//     let gas_service_root_pda = fixture.init_gas_service().await;

//     let gateway_root_pda = fixture
//         .initialize_gateway_config_account(GatewayConfig::default())
//         .await;

//     let interchain_token_service_root_pda =
//         get_interchain_token_service_root_pda(&gateway_root_pda,
// &gas_service_root_pda);

//     let ix =
// interchain_token_service::instruction::build_initialize_instruction(
//         &fixture.payer.pubkey(),
//         &interchain_token_service_root_pda,
//         &gateway_root_pda,
//         &gas_service_root_pda,
//     )
//     .unwrap();
//     let recent_blockhash = fixture.refresh_blockhash().await;
//     let transaction = Transaction::new_signed_with_payer(
//         &[ix],
//         Some(&fixture.payer.pubkey()),
//         &[&fixture.payer],
//         recent_blockhash,
//     );
//     fixture
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     // Setup Mint Account

//     let recent_blockhash = fixture.refresh_blockhash().await;
//     let transaction = Transaction::new_signed_with_payer(
//         &[
//             system_instruction::create_account(
//                 &fixture.payer.pubkey(),
//                 &mint_account.pubkey(),
//                 rent.minimum_balance(Mint::LEN),
//                 Mint::LEN as u64,
//                 &spl_token::id(),
//             ),
//             spl_token::instruction::initialize_mint(
//                 &spl_token::id(),
//                 &mint_account.pubkey(),
//                 &interchain_token_service_root_pda,
//                 None,
//                 0,
//             )
//             .unwrap(),
//         ],
//         Some(&fixture.payer.pubkey()),
//         &[&fixture.payer, &mint_account],
//         recent_blockhash,
//     );
//     fixture
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     // Setup ATA / Destination / To Account // TODO: This supposed to take
// part of     // the Instruction Itself / CPI.

//     let recent_blockhash = fixture.refresh_blockhash().await;
//     let transaction = Transaction::new_signed_with_payer(
//         &[
//
// spl_associated_token_account::instruction::create_associated_token_account(
//                 &fixture.payer.pubkey(),
//                 &wallet_address.pubkey(),
//                 &mint_account.pubkey(),
//                 &spl_token::id(),
//             ),
//         ],
//         Some(&fixture.payer.pubkey()),
//         &[&fixture.payer],
//         recent_blockhash,
//     );
//     fixture
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     let account = fixture
//         .banks_client
//         .get_account(associated_token_account)
//         .await
//         .unwrap()
//         .unwrap();
//     let token_account = Account::unpack(&account.data).unwrap();

//     assert_eq!(token_account.owner, wallet_address.pubkey());
//     assert_eq!(token_account.mint, mint_account.pubkey());

//     // // Final Part: GiveToken

//     let ix =
// interchain_token_service::instruction::build_give_token_instruction(
//         TokenManagerType::MintBurn,
//         amount,
//         fixture.payer.pubkey(),
//         mint_account.pubkey(),
//         token_manager.pubkey(),
//         wallet_address.pubkey(),  // owner of "to" account
//         associated_token_account, // previously "to" / ATA
//         interchain_token_service_root_pda,
//         gateway_root_pda,
//         gas_service_root_pda,
//     )?;

//     let transaction: Transaction = Transaction::new_signed_with_payer(
//         &[ix],
//         Some(&fixture.payer.pubkey()),
//         &[&fixture.payer],
//         recent_blockhash,
//     );
//     fixture
//         .banks_client
//         .process_transaction(transaction)
//         .await
//         .unwrap();

//     let account = fixture
//         .banks_client
//         .get_account(associated_token_account)
//         .await
//         .unwrap()
//         .unwrap();
//     let token_account = Account::unpack(&account.data).unwrap();

//     assert_eq!(token_account.amount, amount);
//     assert_ne!(token_account.amount, amount + 1);

//     Ok(())
// }
