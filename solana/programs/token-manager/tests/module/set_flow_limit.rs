use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;
use test_fixtures::account::CheckValidPDAInTests;

#[tokio::test]
async fn test_set_flow_limit() {
    const NEW_FLOW_LIMIT: u64 = 100;
    let flow_limit = 500;
    let mut fixture = super::utils::TestFixture::new().await;
    let mint_authority = Keypair::new();
    let token_mint = fixture.init_new_mint(mint_authority.pubkey()).await;
    let token_manager_pda_pubkey = fixture.setup_token_manager(flow_limit, token_mint).await;

    let ix = token_manager::instruction::build_set_flow_limit_instruction(
        &token_manager_pda_pubkey,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        NEW_FLOW_LIMIT,
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &fixture.flow_repr.operator],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
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
            flow_limit: NEW_FLOW_LIMIT,
            associated_token_account: get_associated_token_address(
                &token_manager_pda_pubkey,
                &token_mint
            ),
            token_mint,
        }
    );
}
