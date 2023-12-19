use solana_program::clock::Clock;
use solana_program::program_pack::Pack;
use solana_program_test::{tokio, BanksClientError};
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use token_manager::instruction::FlowToAdd;
use token_manager::{get_token_flow_account, CalculatedEpoch};

#[tokio::test]
async fn test_add_flow() {
    let flow_limit = 500;
    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp;

    let token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 90,
            add_flow_out: 5,
        },
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
    let token_flow_pda = fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(token_flow_pda.owner, token_manager::id());
    assert_eq!(
        token_flow_pda.data.len(),
        token_manager::state::FlowInOutAccount::LEN
    );
    let token_flow_pda =
        token_manager::state::FlowInOutAccount::unpack_from_slice(&token_flow_pda.data).unwrap();
    assert_eq!(
        token_flow_pda,
        token_manager::state::FlowInOutAccount {
            flow_in: 90,
            flow_out: 5,
        }
    );
}

#[tokio::test]
async fn test_add_flow_2_times() {
    let flow_limit = 500;

    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp;

    let token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 90,
            add_flow_out: 5,
        },
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone(), ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &fixture.flow_repr.operator],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );

    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();
    let token_flow_pda = fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(token_flow_pda.owner, token_manager::id());
    assert_eq!(
        token_flow_pda.data.len(),
        token_manager::state::FlowInOutAccount::LEN
    );
    let token_flow_pda =
        token_manager::state::FlowInOutAccount::unpack_from_slice(&token_flow_pda.data).unwrap();
    assert_eq!(
        token_flow_pda,
        token_manager::state::FlowInOutAccount {
            flow_in: 180,
            flow_out: 10,
        }
    );
}

#[tokio::test]
async fn test_add_flow_old_pdas() {
    let flow_limit = 500;
    let block_timestamp = 10; // super old timestamp
    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 90,
            add_flow_out: 5,
        },
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &fixture.flow_repr.operator],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );

    let res = fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    assert!(matches!(res, BanksClientError::TransactionError(_)));
    assert!(fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .is_none());
}

#[tokio::test]
async fn test_add_flow_in_exceeds_limit() {
    let flow_limit = 500;
    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp;

    let token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: flow_limit - 1,
            add_flow_out: 0,
        },
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
    let ix2 = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: flow_limit + 1,
            add_flow_out: 0,
        },
    )
    .unwrap();

    let transaction = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &fixture.flow_repr.operator],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );

    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();

    let token_flow_pda = fixture
        .banks_client
        .get_account(token_flow_pda)
        .await
        .expect("get_account")
        .expect("account not none");
    assert_eq!(token_flow_pda.owner, token_manager::id());
    assert_eq!(
        token_flow_pda.data.len(),
        token_manager::state::FlowInOutAccount::LEN
    );
    let token_flow_pda =
        token_manager::state::FlowInOutAccount::unpack_from_slice(&token_flow_pda.data).unwrap();
    assert_eq!(
        token_flow_pda,
        token_manager::state::FlowInOutAccount {
            flow_in: flow_limit - 1,
            flow_out: 0,
        }
    );
}

#[tokio::test]
async fn test_add_flow_in_works_fine() {
    let flow_limit = 5;
    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 1,
            add_flow_out: 0,
        },
    )
    .unwrap();
    for idx in 0..flow_limit {
        let transaction = Transaction::new_signed_with_payer(
            &[ix.clone()],
            Some(&fixture.payer.pubkey()),
            &[&fixture.payer, &fixture.flow_repr.operator],
            fixture.banks_client.get_latest_blockhash().await.unwrap(),
        );
        fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // sleep for 500ms
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let token_flow_pda = fixture
            .banks_client
            .get_account(token_flow_pda)
            .await
            .expect("get_account")
            .expect("account not none");
        let token_flow_pda =
            token_manager::state::FlowInOutAccount::unpack_from_slice(&token_flow_pda.data)
                .unwrap();

        assert_eq!(
            token_flow_pda,
            token_manager::state::FlowInOutAccount {
                flow_in: idx + 1,
                flow_out: 0,
            }
        );
    }

    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone()],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &fixture.flow_repr.operator],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
}

#[tokio::test]
async fn test_add_flow_out_works_fine() {
    let flow_limit = 5;
    let (mut fixture, token_manager_pda) = super::utils::TestFixture::new()
        .await
        .post_setup(flow_limit)
        .await;

    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp;
    let token_flow_pda = get_token_flow_account(
        &token_manager_pda,
        CalculatedEpoch::new_with_timestamp(block_timestamp as u64),
    );
    let ix = token_manager::instruction::build_add_flow_instruction(
        &fixture.payer.pubkey(),
        &token_manager_pda,
        &token_flow_pda,
        &fixture.flow_repr.operator_group_pda,
        &fixture.flow_repr.init_operator_pda_acc,
        &fixture.flow_repr.operator.pubkey(),
        &fixture.operator_repr.operator_group_pda,
        &fixture.service_program_pda.pubkey(),
        FlowToAdd {
            add_flow_in: 0,
            add_flow_out: 1,
        },
    )
    .unwrap();
    for idx in 0..flow_limit {
        let transaction = Transaction::new_signed_with_payer(
            &[ix.clone()],
            Some(&fixture.payer.pubkey()),
            &[&fixture.payer, &fixture.flow_repr.operator],
            fixture.banks_client.get_latest_blockhash().await.unwrap(),
        );
        fixture
            .banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        // sleep for 500ms
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let token_flow_pda = fixture
            .banks_client
            .get_account(token_flow_pda)
            .await
            .expect("get_account")
            .expect("account not none");
        let token_flow_pda =
            token_manager::state::FlowInOutAccount::unpack_from_slice(&token_flow_pda.data)
                .unwrap();

        assert_eq!(
            token_flow_pda,
            token_manager::state::FlowInOutAccount {
                flow_in: 0,
                flow_out: idx + 1,
            }
        );
    }

    let transaction = Transaction::new_signed_with_payer(
        &[ix.clone()],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer, &fixture.flow_repr.operator],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap_err();
}
