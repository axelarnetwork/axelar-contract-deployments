use solana_program::program_option::COption;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use test_fixtures::account::CheckValidPDAInTests;
use token_manager::get_token_manager_account;

#[tokio::test]
async fn test_token_manager_setup() {
    let mut fixture = super::utils::TestFixture::new().await;
    let mint_authority = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();

    let token_manager_pda_pubkey = get_token_manager_account(
        &fixture.operator_repr.operator_group_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
    );
    let ix = token_manager::instruction::build_setup_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda_pubkey,
        &fixture.operator_repr.operator_group_pda,
        &fixture.operator_repr.operator.pubkey(),
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.service_program_pda.pubkey(),
        &token_mint,
        token_manager::instruction::Setup { flow_limit: 500 },
    )
    .unwrap();
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
            flow_limit: 500,
            associated_token_account: get_associated_token_address(
                &token_manager_pda_pubkey,
                &token_mint
            ),
            token_mint
        }
    );
    let token_manager_ata = fixture
        .banks_client
        .get_account(get_associated_token_address(
            &token_manager_pda_pubkey,
            &token_mint,
        ))
        .await
        .expect("get_account")
        .expect("account not none");

    let data = spl_token::state::Account::unpack(token_manager_ata.data.as_slice()).unwrap();
    assert_eq!(
        data,
        spl_token::state::Account {
            mint: token_mint,
            owner: token_manager_pda_pubkey,
            amount: 0,
            delegate: COption::Some(fixture.service_program_pda.pubkey()),
            state: spl_token::state::AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: u64::MAX,
            close_authority: COption::None,
        }
    );
}
