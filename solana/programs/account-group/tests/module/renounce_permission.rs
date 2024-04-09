use account_group::get_permission_account;
use account_group::state::PermissionAccount;
use solana_program_test::{tokio, ProgramTestBanksClientExt};
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use test_fixtures::account::CheckValidPDAInTests;

use crate::utils::TestFixture;

#[tokio::test]
async fn test_renounce_permission() {
    let mut fixture = TestFixture::new().await;

    let operators = vec![Keypair::new(), Keypair::new()];
    let operators = operators
        .into_iter()
        .map(|x| {
            let x_pubkey = x.pubkey();
            (
                x,
                get_permission_account(&fixture.operator_group_pda, &x_pubkey),
            )
        })
        .collect::<Vec<_>>();

    add_and_check_operators(&operators, &mut fixture).await;

    // Renounce all operators
    for (operator, permission_pda) in operators.iter() {
        renounce_permission(&mut fixture, permission_pda, operator).await;
    }

    // Check that all operators were removed
    for (_operator, permission_pda) in operators.iter() {
        let pda_does_not_exist = fixture
            .banks_client
            .get_account(*permission_pda)
            .await
            .expect("get_account")
            .is_none();
        assert!(pda_does_not_exist);
    }

    // Can add again
    add_and_check_operators(&operators, &mut fixture).await;
}

async fn add_and_check_operators(
    operators: &[(Keypair, solana_program::pubkey::Pubkey)],
    fixture: &mut TestFixture,
) {
    // Add new operators
    for (operator, permission_pda) in operators.iter() {
        add_to_group(fixture, operator, permission_pda).await;
    }

    // Check that all operators were added
    for (_operator, permission_pda) in operators.iter() {
        let permission_pda = fixture
            .banks_client
            .get_account(*permission_pda)
            .await
            .expect("get_account")
            .expect("account not none");
        permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)
            .unwrap();
    }
}

async fn renounce_permission(
    fixture: &mut TestFixture,
    permission_pda: &solana_program::pubkey::Pubkey,
    operator: &Keypair,
) {
    let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();
    let recent_blockhash = fixture
        .banks_client
        .get_new_latest_blockhash(&recent_blockhash)
        .await
        .unwrap();

    let ix = account_group::instruction::build_renounce_permission_instruction(
        &fixture.operator_group_pda,
        permission_pda,
        &operator.pubkey(),
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, operator],
        recent_blockhash,
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
}

async fn add_to_group(
    fixture: &mut TestFixture,
    operator: &Keypair,
    permission_pda: &solana_program::pubkey::Pubkey,
) {
    let recent_blockhash = fixture.banks_client.get_latest_blockhash().await.unwrap();

    let ix = account_group::instruction::build_add_account_to_group_instruction(
        &fixture.payer.pubkey(),
        &fixture.operator_group_pda,
        &fixture.init_operator_pda_acc,
        &fixture.init_operator.pubkey(),
        &operator.pubkey(),
        permission_pda,
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
