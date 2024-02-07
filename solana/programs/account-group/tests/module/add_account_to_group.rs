use account_group::get_permission_account;
use account_group::state::PermissionAccount;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;

use crate::utils::TestFixture;

#[tokio::test]
async fn test_add_operator() {
    let mut fixture = TestFixture::new().await;

    let operators = vec![Keypair::new(), Keypair::new()];

    for operator in operators.iter() {
        let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();

        let op_acc = get_permission_account(&fixture.operator_group_pda, &operator.pubkey());
        let ix = account_group::instruction::build_add_account_to_group_instruction(
            &fixture.payer.pubkey(),
            &fixture.operator_group_pda,
            &fixture.init_operator_pda_acc,
            &fixture.init_operator.pubkey(),
            &operator.pubkey(),
            &op_acc,
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&fixture.payer.pubkey()),
            &[&fixture.payer, &fixture.init_operator],
            recent_blockhash,
        );
        fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }

    // Check that all operators were added
    for operator in operators.iter() {
        let op_acc = get_permission_account(&fixture.operator_group_pda, &operator.pubkey());
        let op_acc = fixture
            .banks_client
            .get_account(op_acc)
            .await
            .expect("get_account")
            .expect("account not none");
        op_acc
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)
            .unwrap();
    }
}
