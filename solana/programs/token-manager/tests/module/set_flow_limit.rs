use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

#[tokio::test]
async fn test_set_flow_limit() {
    const NEW_FLOW_LIMIT: u64 = 100;
    let flow_limit = 500;
    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let ix = token_manager::instruction::build_set_flow_limit_instruction(
        &token_manager_pda,
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
        .get_account(token_manager_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(token_manager_pda.owner, token_manager::id());
    assert_eq!(
        token_manager_pda.data.len(),
        token_manager::state::TokenManagerAccount::LEN
    );
    let token_manager_account =
        token_manager::state::TokenManagerAccount::unpack_from_slice(&token_manager_pda.data)
            .unwrap();
    assert_eq!(
        token_manager_account,
        token_manager::state::TokenManagerAccount {
            flow_limit: NEW_FLOW_LIMIT,
        }
    );
}
