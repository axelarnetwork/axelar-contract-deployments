use solana_program::clock::Clock;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use token_manager::{get_token_flow_account, get_token_manager_account, CalculatedEpoch};

#[tokio::test]
async fn test_setup() {
    let mut fixture = super::utils::TestFixture::new().await;
    let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();

    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp;

    let token_manager_pda = get_token_manager_account(
        &fixture.operator_repr.operator_group_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
    );
    let _token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_setup_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &fixture.operator_repr.operator_group_pda,
        &fixture.operator_repr.init_operator_pda_acc,
        &fixture.operator_repr.operator.pubkey(),
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.service_program_pda.pubkey(),
        token_manager::instruction::Setup {
            operator_group_id: fixture.operator_repr.operator_group_id.clone(),
            flow_limiter_group_id: fixture.flow_repr.operator_group_id.clone(),
            flow_limit: 500,
        },
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[
            &fixture.payer,
            &fixture.operator_repr.operator,
            &fixture.flow_repr.operator,
        ],
        recent_blockhash,
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
        token_manager::state::TokenManagerAccount { flow_limit: 500 }
    );
}
