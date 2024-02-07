use account_group::instruction::GroupId;
use account_group::state::{PermissionAccount, PermissionGroupAccount};
use account_group::{get_permission_account, get_permission_group_account};
use solana_program_test::tokio;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;

use crate::utils::program_test;

#[tokio::test]
async fn test_create_permission_group() {
    let user = Keypair::new();
    let permission_group_id = GroupId::new("test-operation-chain-id");
    let permission_group_pda = get_permission_group_account(&permission_group_id);
    let permission_pda_acc = get_permission_account(&permission_group_pda, &user.pubkey());
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    // Associated account does not exist
    assert_eq!(
        banks_client
            .get_account(permission_group_pda)
            .await
            .expect("get_account"),
        None,
    );
    assert_eq!(
        banks_client
            .get_account(permission_pda_acc)
            .await
            .expect("get_account"),
        None,
    );

    let ix = account_group::instruction::build_setup_permission_group_instruction(
        &payer.pubkey(),
        &permission_group_pda,
        &permission_pda_acc,
        &user.pubkey(),
        permission_group_id.clone(),
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // permission group account now exists
    let op_chain_acc = banks_client
        .get_account(permission_group_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    let group = op_chain_acc
        .check_initialized_pda::<PermissionGroupAccount>(&account_group::id())
        .unwrap();

    assert_eq!(group.id, permission_group_id);
    // Operator account now exists
    let permissioned_user_ac = banks_client
        .get_account(permission_pda_acc)
        .await
        .expect("get_account")
        .expect("account not none");
    let _acc = permissioned_user_ac
        .check_initialized_pda::<PermissionAccount>(&account_group::id())
        .unwrap();
}
