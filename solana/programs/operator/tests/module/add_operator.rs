use operator::get_operator_account;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use crate::utils::TestFixture;

#[tokio::test]
async fn test_add_operator() {
    let mut fixture = TestFixture::new().await;

    let operators = vec![
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
        Keypair::new(),
    ];

    for operator in operators.iter() {
        let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();

        let op_acc = get_operator_account(&fixture.operator_group_pda, &operator.pubkey());
        let ix = operator::instruction::build_add_operator_instruction(
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
        let op_acc = get_operator_account(&fixture.operator_group_pda, &operator.pubkey());
        let op_acc = fixture
            .banks_client
            .get_account(op_acc)
            .await
            .expect("get_account")
            .expect("account not none");
        let op_acc = operator::state::OperatorAccount::unpack_from_slice(&op_acc.data).unwrap();
        assert!(op_acc.is_active());
    }
}
