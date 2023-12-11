use operator::{get_operator_account, get_operator_group_account};
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;

use crate::utils::program_test;

#[tokio::test]
async fn test_create_operator_group() {
    let operator = Keypair::new();
    let operator_group_id = "test-operation-chain-id";
    let op_chain_acc = get_operator_group_account(operator_group_id);
    let op_acc = get_operator_account(&op_chain_acc, &operator.pubkey());
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    // Associated account does not exist
    assert_eq!(
        banks_client
            .get_account(op_chain_acc)
            .await
            .expect("get_account"),
        None,
    );
    assert_eq!(
        banks_client.get_account(op_acc).await.expect("get_account"),
        None,
    );

    let ix = operator::instruction::build_create_group_instruction(
        &payer.pubkey(),
        &op_chain_acc,
        &op_acc,
        &operator.pubkey(),
        operator_group_id.to_string(),
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer, &operator],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // operator chain account now exists
    let op_chain_acc = banks_client
        .get_account(op_chain_acc)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(op_chain_acc.owner, operator::id());
    assert_eq!(
        op_chain_acc.data.len(),
        operator::state::OperatorGroupAccount::LEN
    );

    // Operator account now exists
    let op_acc = banks_client
        .get_account(op_acc)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(op_acc.owner, operator::id());
    assert_eq!(op_acc.data.len(), operator::state::OperatorAccount::LEN);
}
