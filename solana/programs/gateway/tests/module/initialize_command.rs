use axelar_message_primitives::DestinationProgramId;
use gmp_gateway::state::{
    GatewayApprovedCommand, GatewayCommandStatus, TransferOperatorship, ValidateContractCall,
};
use itertools::Either;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::pubkey::Pubkey;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::{prepare_execute_data, TestFixture};

use crate::{example_payload, example_worker_set, gateway_approved_command_ixs, program_test};

#[tokio::test]
async fn succesfully_initialize_validate_contract_call_command() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_execute_data_pubkey, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
            ],
            &operators,
            quorum,
        )
        .await;

    // Action
    let ixs = gateway_approved_command_ixs(execute_data, gateway_root_pda, &fixture);
    let gateway_approved_command_pdas = ixs.iter().map(|(pda, _)| *pda).collect::<Vec<_>>();
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Assert
    for pda in gateway_approved_command_pdas {
        let account = fixture
            .banks_client
            .get_account(pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_contract_call_approved());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Pending)
        ));
    }
}

#[tokio::test]
async fn succesfully_initialize_transfer_operatorship_message() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let (_execute_data_pubkey, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            // Every worker set is slightly different to prevent hash collisions because there's no
            // random data
            &[
                Either::Right(example_worker_set(42, 42)),
                Either::Right(example_worker_set(43, 43)),
                Either::Right(example_worker_set(44, 44)),
            ],
            &operators,
            quorum,
        )
        .await;

    // Action
    let ixs = gateway_approved_command_ixs(execute_data, gateway_root_pda, &fixture);
    let gateway_approved_command_pdas = ixs.iter().map(|(pda, _)| *pda).collect::<Vec<_>>();
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Assert
    for pda in gateway_approved_command_pdas {
        let account = fixture
            .banks_client
            .get_account(pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_contract_call_approved());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::TransferOperatorship(TransferOperatorship::Pending)
        ));
    }
}

#[tokio::test]
async fn succesfully_initialize_transfer_operatorship_message_together_with_call_contract() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_execute_data_pubkey, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            // Every worker set is slightly different to prevent hash collisions because there's no
            // random data
            &[
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
                Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
                Either::Right(example_worker_set(42, 42)),
                Either::Right(example_worker_set(43, 44)),
                Either::Right(example_worker_set(44, 44)),
            ],
            &operators,
            quorum,
        )
        .await;

    // Action
    let ixs = gateway_approved_command_ixs(execute_data, gateway_root_pda, &fixture);
    let pdas = ixs.iter().map(|(pda, _)| *pda).collect::<Vec<_>>();
    let (call_contract_ops, transfer_ops) = pdas.split_at(3);
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Assert
    for pda in call_contract_ops {
        let account = fixture
            .banks_client
            .get_account(*pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_contract_call_approved());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Pending)
        ));
    }
    for pda in transfer_ops {
        let account = fixture
            .banks_client
            .get_account(*pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_contract_call_approved());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::TransferOperatorship(TransferOperatorship::Pending)
        ));
    }
}

#[tokio::test]
async fn fail_when_gateway_root_pda_not_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = Pubkey::new_unique();
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let gateway_execute_data = prepare_execute_data(
        &[Either::Left(
            custom_message(destination_program_id, example_payload()).unwrap(),
        )],
        &operators,
        quorum,
        &gateway_root_pda,
    );

    // Action
    let ixs = gateway_approved_command_ixs(gateway_execute_data.0, gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

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
async fn succesfully_initialize_command_which_belongs_to_a_different_execute_data_set() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_execute_data_pubkey_1, _execute_data_1, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[Either::Left(
                custom_message(destination_program_id, example_payload()).unwrap(),
            )],
            &operators,
            quorum,
        )
        .await;
    let gateway_execute_data_2 = prepare_execute_data(
        &[Either::Left(
            custom_message(destination_program_id, example_payload()).unwrap(),
        )],
        &operators,
        quorum,
        &gateway_root_pda,
    );
    // Action
    let ixs = gateway_approved_command_ixs(gateway_execute_data_2.0, gateway_root_pda, &fixture);
    let pdas = ixs.iter().map(|(pda, _)| *pda).collect::<Vec<_>>();
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Assert
    for pda in pdas {
        let account = fixture
            .banks_client
            .get_account(pda)
            .await
            .expect("call failed")
            .expect("account not found");
        let gateway_approved_command = account
            .check_initialized_pda::<GatewayApprovedCommand>(&gmp_gateway::id())
            .unwrap();
        assert!(!gateway_approved_command.is_command_executed());
        assert!(!gateway_approved_command.is_contract_call_approved());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::ValidateContractCall(ValidateContractCall::Pending)
        ));
    }
}

#[tokio::test]
async fn fail_when_validate_contract_call_already_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (_execute_data_pubkey, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[Either::Left(
                custom_message(destination_program_id, example_payload()).unwrap(),
            )],
            &operators,
            quorum,
        )
        .await;

    let ixs = gateway_approved_command_ixs(execute_data, gateway_root_pda, &fixture);
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Action -- will fail when trying to initialize the same command
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

    // Assert
    //
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // this means that the account was already initialized
        // TODO: improve error message
        .any(|x| x.contains("invalid account data for instruction")),);
}

#[tokio::test]
async fn fail_when_transfer_operatorship_is_already_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let new_worker_set = example_worker_set(42, 43);
    let (_execute_data_pubkey, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[Either::Right(new_worker_set)],
            &operators,
            quorum,
        )
        .await;

    let ixs = gateway_approved_command_ixs(execute_data, gateway_root_pda, &fixture);
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Action -- will fail when trying to initialize the same command
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

    // Assert
    //
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // this means that the account was already initialized
        // TODO: improve error message
        .any(|x| x.contains("invalid account data for instruction")),);
}

/// The [WorkerSet data structure](https://github.com/axelarnetwork/axelar-amplifier/blob/a68eb5b3c28d9f6c0bd665ba012cbec13970f3a8/contracts/multisig/src/worker_set.rs#L10-L20) has this comment written for the `created_at` field:
/// ```rust
/// // for hash uniqueness. The same exact worker set could be in use at two different times,
/// // and we need to be able to distinguish between the two
/// pub created_at: u64,
/// ```
/// But realistically this field gets dropped when it's encoded via bcs or abi
/// into the `Operators` structure. [link to abi encoding](https://github.com/axelarnetwork/axelar-amplifier/blob/a68eb5b3c28d9f6c0bd665ba012cbec13970f3a8/contracts/multisig-prover/src/encoding/abi.rs#L133-L146)
/// This means that if we change the `created_at` field, the hash of the
/// `WorkerSet` WILL NOT change.
#[tokio::test]
async fn fail_when_transfer_operatorship_has_unchanged_block_height() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    let initial_created_at = 180;
    let new_created_at = 360;
    let new_worker_set = example_worker_set(111, initial_created_at);
    let same_worker_set_with_different_created_at = {
        let mut tmp = new_worker_set.clone();
        tmp.created_at = new_created_at;
        tmp
    };
    let (_execute_data_pubkey_1, execute_data_1, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[Either::Right(new_worker_set)],
            &operators,
            quorum,
        )
        .await;
    let (_execute_data_pubkey_2, execute_data_2, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &[Either::Right(same_worker_set_with_different_created_at)],
            &operators,
            quorum,
        )
        .await;
    let ixs = gateway_approved_command_ixs(execute_data_1, gateway_root_pda, &fixture);
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    fixture.send_tx(&ixs).await;

    // Action -- will fail because the `created_at` field gets dropped when encoded
    // resulting in the same hash for the command
    let ixs = gateway_approved_command_ixs(execute_data_2, gateway_root_pda, &fixture);
    let ixs = ixs.into_iter().map(|(_, ix)| ix).collect::<Vec<_>>();
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs).await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        // this means that the account was already initialized
        // TODO: improve error message
        .any(|x| x.contains("invalid account data for instruction")),);
}
