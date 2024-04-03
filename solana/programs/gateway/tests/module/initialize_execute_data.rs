use axelar_message_primitives::DestinationProgramId;
use cosmwasm_std::Uint256;
use gmp_gateway::state::{GatewayConfig, GatewayExecuteData};
use itertools::Either;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata, ProgramTestBanksClientExt};
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::system_program;
use test_fixtures::axelar_message::{custom_message, new_worker_set};
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::{prepare_execute_data, TestFixture};

use crate::{example_payload, program_test};

#[tokio::test]
async fn test_successfylly_initialize_execute_data() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let quorum = 14;
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (gateway_execute_data, raw_execute_data) = prepare_execute_data(
        &[Either::Left(
            custom_message(destination_program_id, example_payload()).unwrap(),
        )],
        &operators,
        quorum,
        &gateway_config_pda,
    );
    let (execute_data_pda, _, _) = gateway_execute_data.pda(&gateway_config_pda);

    // Action
    fixture
        .send_tx(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            gateway_config_pda,
            raw_execute_data.clone(),
        )
        .unwrap()
        .0])
        .await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(execute_data_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_execute_data =
        borsh::from_slice::<GatewayExecuteData>(&account.data).unwrap();
    assert_eq!(deserialized_gateway_execute_data, gateway_execute_data);
}

#[tokio::test]
async fn test_succesfully_initialize_transfer_operatorship() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let new_operators = vec![
        create_signer_with_weight(33).unwrap(),
        create_signer_with_weight(150).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let (gateway_execute_data, raw_execute_data) = prepare_execute_data(
        &[Either::Right(new_worker_set(
            &new_operators,
            42,
            Uint256::from_u128(42),
        ))],
        &operators,
        quorum,
        &gateway_root_pda,
    );
    let (execute_data_pda, _, _) = gateway_execute_data.pda(&gateway_root_pda);

    // Action
    fixture
        .send_tx(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            gateway_root_pda,
            raw_execute_data.clone(),
        )
        .unwrap()
        .0])
        .await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(execute_data_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_execute_data =
        borsh::from_slice::<GatewayExecuteData>(&account.data).unwrap();
    assert_eq!(deserialized_gateway_execute_data, gateway_execute_data);
}

#[tokio::test]
async fn test_succesfully_initialize_transfer_operatorship_message_together_with_call_contract() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let new_operators = vec![
        create_signer_with_weight(33).unwrap(),
        create_signer_with_weight(150).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (gateway_execute_data, raw_execute_data) = prepare_execute_data(
        &[
            Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
            Either::Right(new_worker_set(&new_operators, 42, Uint256::from_u128(42))),
        ],
        &operators,
        quorum,
        &gateway_root_pda,
    );
    let (execute_data_pda, _, _) = gateway_execute_data.pda(&gateway_root_pda);

    // Action
    fixture
        .send_tx(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            gateway_root_pda,
            raw_execute_data.clone(),
        )
        .unwrap()
        .0])
        .await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(execute_data_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_execute_data =
        borsh::from_slice::<GatewayExecuteData>(&account.data).unwrap();
    assert_eq!(deserialized_gateway_execute_data, gateway_execute_data);
}

#[tokio::test]
async fn test_fail_on_invalid_root_pda() {
    // Setup
    let fake_gateway_root_pda = Pubkey::new_unique();
    let mut program_test = program_test();
    program_test.add_account(
        fake_gateway_root_pda,
        Account {
            lamports: 9999999,
            data: vec![],
            owner: gmp_gateway::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    let mut fixture = TestFixture::new(program_test).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let quorum = 14;
    let _gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_gateway_execute_data, raw_execute_data) = prepare_execute_data(
        &[Either::Left(
            custom_message(destination_program_id, example_payload()).unwrap(),
        )],
        &operators,
        quorum,
        &fake_gateway_root_pda,
    );

    // Action
    let BanksTransactionResultWithMetadata { metadata, result } = fixture
        .send_tx_with_metadata(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            fake_gateway_root_pda,
            raw_execute_data.clone(),
        )
        .unwrap()
        .0])
        .await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // Invalid data stored in the gateway root PDA
        .any(|x| x.contains("invalid account data for instruction")),);
}

#[tokio::test]
async fn test_fail_on_invalid_root_pda_owned_by_system_program() {
    // Setup
    let fake_gateway_root_pda = Pubkey::new_unique();
    let mut program_test = program_test();
    program_test.add_account(
        fake_gateway_root_pda,
        Account {
            lamports: 9999999,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    );
    let mut fixture = TestFixture::new(program_test).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let quorum = 14;
    let _gateway_config_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_gateway_execute_data, raw_execute_data) = prepare_execute_data(
        &[Either::Left(
            custom_message(destination_program_id, example_payload()).unwrap(),
        )],
        &operators,
        quorum,
        &fake_gateway_root_pda,
    );
    // Action

    let BanksTransactionResultWithMetadata { metadata, result } = fixture
        .send_tx_with_metadata(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            fake_gateway_root_pda,
            raw_execute_data.clone(),
        )
        .unwrap()
        .0])
        .await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // We expected the root pda to be owned by the gateway program not something else
        .any(|x| x.contains("Provided owner is not allowed")),);
}

#[tokio::test]
async fn test_fail_on_uninitialized_root_pda() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let quorum = 14;
    let (uninitialized_gateway_config_pda, _bump) = GatewayConfig::pda();
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_gateway_execute_data, raw_execute_data) = prepare_execute_data(
        &[Either::Left(
            custom_message(destination_program_id, example_payload()).unwrap(),
        )],
        &operators,
        quorum,
        &uninitialized_gateway_config_pda,
    );

    // Action
    let BanksTransactionResultWithMetadata { metadata, result } = fixture
        .send_tx_with_metadata(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            uninitialized_gateway_config_pda,
            raw_execute_data.clone(),
        )
        .unwrap()
        .0])
        .await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // This means that the account was not initialized - has 0 lamports
        .any(|x| x.contains("insufficient funds for instruction")),);
}

#[tokio::test]
async fn test_fail_on_already_initialized_execute_data_account() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;

    // Action
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    // We init the execute data account (the helper method sends a tx to the
    // gateway program)
    let (_execute_data_pda, _execute_data, raw_data) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[Either::Left(
                custom_message(destination_program_id, example_payload()).unwrap(),
            )],
            &operators,
            quorum,
        )
        .await;
    // We try to init the execute data account again with the same data
    let BanksTransactionResultWithMetadata { metadata, result } = fixture
        .send_tx_with_metadata(&[gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            gateway_root_pda,
            raw_data,
        )
        .unwrap()
        .0])
        .await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|x| x.contains("invalid account data for instruction")),);
}

/// processing any more than 19 operators results in `memory allocation failed,
/// out of memory` Which means that we exceeded the 32kb heap memory limit
/// [docs](https://solana.com/docs/programs/faq#heap-size)
///
/// Technically we could try using a custom allocator to clean up the heap
/// because we still have a lot of compute budget to work with:
/// `consumed 690929 of 1399850 compute units` - on 33 operator amount
///
/// 1399850 - this is the maximum amount of compute units that we can use, if we
/// try setting a larger value, it just gets rounded to this one.
#[tokio::test]
async fn test_size_limits_for_different_operators() {
    // Setup
    for amount_of_operators in [2, 4, 8, 16, 17, 18, 19] {
        dbg!(amount_of_operators);
        let operators = (0..amount_of_operators)
            .map(|x| create_signer_with_weight(x + 1).unwrap())
            .collect::<Vec<_>>();
        let quorum =
            ((0..amount_of_operators as i64).sum::<i64>() + amount_of_operators as i64) as u128;
        let mut fixture = TestFixture::new(program_test()).await;
        let gateway_root_pda = fixture
            .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
            .await;

        let destination_program_id = DestinationProgramId(Pubkey::new_unique());
        let (_gateway_execute_data, raw_execute_data) = prepare_execute_data(
            &[Either::Left(
                custom_message(destination_program_id, example_payload()).unwrap(),
            )],
            &operators,
            quorum,
            &gateway_root_pda,
        );
        fixture.recent_blockhash = fixture
            .banks_client
            .get_new_latest_blockhash(&fixture.recent_blockhash)
            .await
            .unwrap();
        fixture
            .send_tx(&[
                // add compute budget increase
                ComputeBudgetInstruction::set_compute_unit_limit(1399850_u32),
                gmp_gateway::instructions::initialize_execute_data(
                    fixture.payer.pubkey(),
                    gateway_root_pda,
                    raw_execute_data.clone(),
                )
                .unwrap()
                .0,
            ])
            .await;
    }
}

/// Any more than 16 *small* messages results in `memory allocation failed,
/// out of memory` (with only a single opeator who signed the batch)
///
/// consumed 651675 of 1399700 compute units
///
/// 1399850 - this is the maximum amount of compute units that we can use, if we
/// try setting a larger value, it just gets rounded to this one.
#[tokio::test]
async fn test_message_limits_with_different_amounts() {
    // Setup
    for amount_of_messages in [1, 2, 4, 8, 16] {
        dbg!(amount_of_messages);
        let operators = vec![create_signer_with_weight(4).unwrap()];
        let quorum = 4;
        let mut fixture = TestFixture::new(program_test()).await;
        let gateway_root_pda = fixture
            .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
            .await;

        let destination_program_id = DestinationProgramId(Pubkey::new_unique());
        let (_gateway_execute_data, raw_execute_data) = prepare_execute_data(
            &vec![
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap());
                amount_of_messages
            ],
            &operators,
            quorum,
            &gateway_root_pda,
        );
        fixture.recent_blockhash = fixture
            .banks_client
            .get_new_latest_blockhash(&fixture.recent_blockhash)
            .await
            .unwrap();
        fixture
            .send_tx(&[
                // add compute budget increase
                ComputeBudgetInstruction::set_compute_unit_limit(1399850_u32),
                gmp_gateway::instructions::initialize_execute_data(
                    fixture.payer.pubkey(),
                    gateway_root_pda,
                    raw_execute_data.clone(),
                )
                .unwrap()
                .0,
            ])
            .await;
    }
}
