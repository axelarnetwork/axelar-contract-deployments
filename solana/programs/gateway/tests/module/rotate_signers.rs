use axelar_message_primitives::command::{DecodedCommand, U256};
use axelar_message_primitives::DestinationProgramId;
use cosmwasm_std::Uint256;
use gmp_gateway::instructions::GatewayInstruction;
use gmp_gateway::state::GatewayConfig;
use itertools::Either;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use test_fixtures::axelar_message::{custom_message, WorkerSetExt};

use crate::{
    create_worker_set, example_payload, get_approved_commmand, get_gateway_events,
    get_gateway_events_from_execute_data, prepare_questionable_execute_data,
    setup_initialised_gateway,
};

/// successfully process execute when there is 1 transfer operatorship commands
#[tokio::test]
async fn successfully_rotates_signers() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let (new_worker_set, new_signers) = create_worker_set(&[500_u128, 200_u128], 700_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await
        .pop()
        .unwrap();

    // Action
    let tx = fixture
        .rotate_signers_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pda,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs =
        get_gateway_events_from_execute_data(&execute_data.command_batch.commands);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    let approved_commmand =
        get_approved_commmand(&mut fixture, &gateway_approved_command_pda).await;
    assert!(approved_commmand.is_command_executed());

    // - operators have been updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(2_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(
        root_pda_data
            .auth_weighted
            .operator_hash_for_epoch(&new_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );

    // - test that both operator sets can sign new messages
    for operator_set in [new_signers, operators] {
        let destination_program_id = DestinationProgramId(Pubkey::new_unique());
        fixture
            .fully_approve_messages(
                &gateway_root_pda,
                &[custom_message(destination_program_id, example_payload()).unwrap()],
                &operator_set,
            )
            .await;
    }
}

/// cannot process when there are more than 1 rotate signers commands
#[tokio::test]
async fn fail_on_processing_rotate_signers_when_there_are_3_commands() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;

    let (new_worker_set_one, _) = create_worker_set(&[11_u128, 22_u128], 10_u128);
    let (new_worker_set_two, _) = create_worker_set(&[33_u128, 44_u128], 10_u128);
    let (new_worker_set_three, _) = create_worker_set(&[55_u128, 66_u128], 10_u128);

    let messages = [
        new_worker_set_one.clone(),
        new_worker_set_two.clone(),
        new_worker_set_three.clone(),
    ]
    .map(Either::Right);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    #[allow(deprecated)]
    let ix = gmp_gateway::instructions::handle_execute_data(
        gateway_root_pda,
        execute_data_pda,
        &gateway_approved_command_pdas,
        gmp_gateway::id(),
        borsh::to_vec(&GatewayInstruction::RotateSigners).unwrap(),
    )
    .unwrap();
    let tx = fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("expected exactly one `RotateSigners` command") }));
}

/// disallow operatorship transfer if any other operator besides the most recent
/// epoch signed the proof
#[tokio::test]
async fn fail_if_rotate_signers_signed_by_old_signer_set() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _new_signers) = create_worker_set(&[500_u128, 200_u128], 700_u128);
    fixture
        .fully_rotate_signers(&gateway_root_pda, new_worker_set.clone(), &operators)
        .await;

    // Action - the transfer ops gets ignored because we use `operators`
    let (newer_worker_set, _newer_signers) = create_worker_set(&[444_u128, 555_u128], 333_u128);
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            newer_worker_set.clone(),
            &operators,
        )
        .await;

    // Assert
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}

/// `transfer_operatorship` is ignored if total weight is smaller than new
/// command weight quorum (tx succeeds)
#[tokio::test]
async fn ignore_transfer_ops_if_total_weight_is_smaller_than_quorum() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[Uint256::one(), Uint256::one()], 10_u128);

    // Action
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            new_worker_set.clone(),
            &operators,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_eq!(gateway.auth_weighted.operators().len(), 1);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

#[tokio::test]
async fn fail_if_order_of_commands_is_not_the_same_as_order_of_accounts() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);

    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;

    // Action
    let mut gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    gateway_approved_command_pdas.reverse();

    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
}

/// `transfer_operatorship` is ignored if new operator len is 0 (tx succeeds)
#[tokio::test]
async fn ignore_transfer_ops_if_new_ops_len_is_zero() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&([] as [u128; 0]), 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);
    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &operators, quorum)
        .await;

    // Action
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await
        .pop()
        .unwrap();
    let tx = fixture
        .rotate_signers_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if new operators are not sorted (tx
/// succeeds)
#[tokio::test]
#[ignore = "cannot implement this without changing the bcs encoding of the `TransferOperatorship` command"]
async fn ignore_transfer_ops_if_new_ops_are_not_sorted() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[555_u128, 678_u128], 10_u128);
    let messages = [new_worker_set.clone()].map(Either::Right);

    let (mut execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    // reverse the operators
    let decoded_command = execute_data.command_batch.commands.get_mut(0).unwrap();
    if let DecodedCommand::RotateSigners(signer_set) = decoded_command {
        signer_set.operators.reverse();
    }

    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            // issue: updating the `execute_data` does not update the `gateway_execute_data_raw`
            // which is what we actually use when encoding the data.
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if operator len does not match weigths
/// len (tx succeeds)
#[tokio::test]
#[ignore = "cannot implement this without changing the bcs encoding of the `TransferOperatorship` command"]
async fn ignore_transfer_ops_if_len_does_not_match_weigh_len() {
    // Setup
    let (mut fixture, quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[555_u128, 678_u128], 10_u128);

    let messages = [new_worker_set.clone()].map(Either::Right);

    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &operators,
        &operators,
        quorum,
        &gateway_root_pda,
    );
    // todo: update the len of operators or weights
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            // issue: updating the `execute_data` does not update the `gateway_execute_data_raw`
            // which is what we actually use when encoding the data.
            &gateway_execute_data_raw,
            &execute_data,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// transfer_operatorship` is ignored if total weights sum exceed u256 max (tx
/// succeeds)
#[tokio::test]
#[ignore = "cannot test because the bcs encoding transforms the u256 to a u128 and fails before we actually get to the on-chain logic"]
async fn ignore_transfer_ops_if_total_weight_sum_exceeds_u256() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) = create_worker_set(&[Uint256::MAX, Uint256::MAX], 10_u128);

    // Action
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            new_worker_set.clone(),
            &operators,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}

/// `transfer_operatorship` is ignored if total weights == 0 (tx succeeds)
#[tokio::test]
#[ignore = "cannot test because the bcs encoding transforms the u256 to a u128 and fails before we actually get to the on-chain logic"]
async fn ignore_transfer_ops_if_total_weight_sum_is_zero() {
    // Setup
    let (mut fixture, _quorum, operators, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let (new_worker_set, _signers) =
        create_worker_set(&[Uint256::zero(), Uint256::zero()], 10_u128);

    // Action
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            new_worker_set.clone(),
            &operators,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = U256::from(1_u8);
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch);
    assert_ne!(
        gateway
            .auth_weighted
            .operator_hash_for_epoch(&constant_epoch)
            .unwrap(),
        &new_worker_set.hash_solana_way(),
    );
}
