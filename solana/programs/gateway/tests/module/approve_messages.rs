use axelar_message_primitives::command::U256;
use axelar_message_primitives::DestinationProgramId;
use cosmwasm_std::Uint256;
use gmp_gateway::state::{GatewayApprovedCommand, GatewayExecuteData};
use itertools::{Either, Itertools};
use multisig::key::Signature;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use test_fixtures::axelar_message::{custom_message, new_signer_set};
use test_fixtures::execute_data::{self, create_command_batch, sign_batch};

use crate::{
    create_signer_set, example_payload, get_approved_command, get_gateway_events,
    get_gateway_events_from_execute_data, prepare_questionable_execute_data,
    setup_initialised_gateway,
};

#[tokio::test]
async fn successfully_process_execute_when_there_are_no_commands() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let messages = [];
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
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

    // Assert
    assert!(tx.result.is_ok())
}

/// successfully process execute when there are 3 validate message
/// commands - emits message approved events
#[tokio::test]
async fn successfully_process_execute_when_there_are_3_validate_message_commands() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
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

    // Assert
    assert!(tx.result.is_ok());
    // - events get emitted
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
    for gateway_approved_command_pda in gateway_approved_command_pdas.iter() {
        let approved_command =
            get_approved_command(&mut fixture, gateway_approved_command_pda).await;
        assert!(approved_command.is_command_approved());
    }
}

/// fail on processing approve messages when there is a rotate signers command
/// in there
#[tokio::test]
async fn fail_on_processing_approve_messages_when_there_is_rotate_signers_command_in_there() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let (new_signer_set, _) = create_signer_set(&[500_u128, 200_u128], 700_u128);
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
        Either::Right(new_signer_set.clone()),
        Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
        Either::Left(custom_message(destination_program_id, example_payload()).unwrap()),
    ];
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Non-approve command provided to 'approve-messages':") }));
}

/// calling the same execute flow multiple times with the same execute data will
/// not "approve" the command twice.
#[tokio::test]
async fn successfully_consumes_repeating_commands_idempotency_same_batch() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let emitted_events = get_gateway_events(&tx);
    assert!(
        emitted_events.is_empty(),
        "no events should be emitted when processing duplicate commands"
    );
}

/// if a given command is a part of another batch and it's been executed, it
/// should be ignored in subsequent batches if its present in those.
#[tokio::test]
async fn successfully_consumes_repeating_commands_idempotency_unique_batches() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 42, 33], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Action
    // - we create a new batch with the old command + a new unique command
    // NOTE: we need to add a new command because otherwise the `execute data` pda
    // will be the same.
    let messages = [
        messages[0].clone().left().unwrap().clone(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pda_new = fixture
        .init_pending_gateway_commands(
            &gateway_root_pda,
            &[execute_data.command_batch.commands[1].clone()],
        )
        .await;
    let gateway_approved_command_pda_new = gateway_approved_command_pda_new[0];
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &[
                gateway_approved_command_pdas[0],
                gateway_approved_command_pda_new,
            ],
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let emitted_events = get_gateway_events(&tx);
    assert_eq!(
        emitted_events.len(),
        1,
        "only a single event should be emitted (first command in the batch is ignored)"
    );
}

/// fail if if root config has no signers
#[tokio::test]
async fn fail_if_gateway_config_has_no_signers_signed_by_unknown_signer_set() {
    // Setup
    let (mut fixture, quorum, _signers, gateway_root_pda) =
        setup_initialised_gateway(&[], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (_new_signer_set, signers) = create_signer_set(&[11_u128, 22_u128], 10_u128);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if if root config has no signers and there are no signatures in the
/// execute data
#[tokio::test]
async fn fail_if_gateway_config_has_no_signers_signed_by_empty_set() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    assert!(execute_data.proof.signatures.is_empty());
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

/// fail if root config not initialised
#[tokio::test]
async fn fail_if_root_config_not_initialised() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let messages = [].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let gateway_root_pda = Pubkey::new_unique();
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("insufficient funds") }));
}

/// fail if execute data not initialized
#[tokio::test]
async fn fail_if_execute_data_not_initialised() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let messages = [].map(Either::Left);
    let (_execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let execute_data_pda = Pubkey::new_unique();
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("insufficient funds") }));
}

/// fail if invalid account for gateway passed (e.g. initialized command)
#[tokio::test]
async fn fail_if_invalid_account_for_gateway() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_approved_command_pdas[0], // should be gateway_root_pda
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("failed to deserialize account") }));
}

/// fail if invalid account for execute data passed (e.g. initialized command)
#[tokio::test]
async fn fail_if_invalid_account_for_execute_data() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (_execute_data_pda, execute_data, _) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &gateway_approved_command_pdas[0], // should be execute_data_pda
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Failed to serialize or deserialize account data") }));
}

/// fail if epoch for signers was not found (inside `validate_proof`)
#[tokio::test]
async fn fail_if_epoch_for_signers_was_not_found() {
    // Setup
    let (_unregistered_signer_set, unregistered_signer_set_signers) =
        create_signer_set(&[55_u128, 66_u128], 10_u128);
    let (mut fixture, quorum, _signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data_pda, execute_data, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            &messages,
            &unregistered_signer_set_signers,
            quorum,
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if signer set epoch is older than 16 epochs away (inside
/// `validate_proof`)
#[tokio::test]
async fn fail_if_signer_set_epoch_is_older_than_16() {
    // We setup a new gateway that has an initial signer set registered.
    let (mut fixture, _quorum, initial_signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let initial_signer_set = new_signer_set(&initial_signers, 0, Uint256::from_u128(33));
    let initial_signer_set = [(initial_signer_set.clone(), initial_signers.clone())];
    // We generate 4 new unique signer sets (not registered yet)
    let new_signer_sets = (1..=4)
        .map(|x| create_signer_set(&[55_u128, x], 55_u128 + x))
        .collect::<Vec<_>>();
    // Only the latest signer set is allowed to call "rotate signers" ix
    // to register the next latest signer set. We iterate over all signer sets,
    // calling "rotate signer" with the last known signer set to register
    // the next latest signer set.
    for (idx, ((_current_signer_set, current_signer_set_signers), (new_signer_set, _))) in
        (initial_signer_set.iter().chain(new_signer_sets.iter()))
            .tuple_windows::<(_, _)>()
            .enumerate()
    {
        let new_epoch = U256::from(idx as u8 + 1_u8);
        let root_pda_data = fixture
            .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
            .await;
        assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);

        fixture
            .fully_rotate_signers(
                &gateway_root_pda,
                new_signer_set.clone(),
                current_signer_set_signers,
            )
            .await;
    }

    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    // Now we have registered 5 sets in total (1 initial signer set + 4 that we
    // generated). The "epoch" is an incremental counter. But the data structure
    // only kept around 4 entries.
    let new_epoch = U256::from(5_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(root_pda_data.auth_weighted.signer_sets().len(), 4);
    // Action
    // Any of the lastest 4 signer sets are allowed to "approve messages" coming
    // Axelar->Solana direction.
    for (_, signer_set) in new_signer_sets.iter() {
        let destination_program_id = DestinationProgramId(Pubkey::new_unique());
        fixture
            .fully_approve_messages(
                &gateway_root_pda,
                &[custom_message(destination_program_id, example_payload()).unwrap()],
                signer_set,
            )
            .await;
    }

    // We cannot use the first signer set anymore.
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(
            &gateway_root_pda,
            &[custom_message(destination_program_id, example_payload()).unwrap()],
            &initial_signers,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if signatures cannot be recovered (inside `validate_signatures`
/// ProofError::Secp256k1RecoverError)
#[tokio::test]
async fn fail_if_invalid_signatures() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);

    let command_batch = create_command_batch(&messages).unwrap();
    let signatures = {
        // intentionally mangle the signature so it cannot be recovered
        let mut signatures = sign_batch(&command_batch, &signers).unwrap();
        signatures.iter_mut().for_each(|x| {
            x.as_mut().map(|x| {
                let fake_signature = vec![3u8; 65];
                let fake_signature = cosmwasm_std::HexBinary::from(fake_signature.as_slice());
                let fake_signature = Signature::EcdsaRecoverable(
                    multisig::key::Recoverable::try_from(fake_signature).unwrap(),
                );
                *x = fake_signature;

                x
            });
        });
        signatures
    };
    let encoded_message =
        execute_data::encode(&command_batch, signers.to_vec(), signatures, quorum).unwrap();
    let execute_data =
        GatewayExecuteData::new(encoded_message.as_ref(), &gateway_root_pda).unwrap();
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &encoded_message, &execute_data)
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(Secp256k1RecoverError(InvalidSignature))") }));
}

/// fail if invalid signer set signed the command batch (inside
/// `validate_signatures` ProofError::LowSignatureWeight)
#[tokio::test]
async fn fail_if_invalid_signer_set_signed_command_batch() {
    // Setup
    let unregistered_signer_set_signer = create_signer_set(&[66_u128], 10_u128).1[0].clone();
    let (mut fixture, quorum, signer_set, gateway_root_pda) =
        setup_initialised_gateway(&[11], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let signing_signers = vec![unregistered_signer_set_signer];
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signing_signers,
        &signer_set,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

/// fail if small subset signers signed the command batch (inside
/// `validate_signatures` ProofError::LowSignatureWeight)
#[tokio::test]
async fn fail_if_subset_without_expected_weight_signed_batch() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let signing_signers = vec![signers[0].clone(), signers[1].clone()]; // subset of the signer set
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signing_signers,
        &signers,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

/// succeed if the larger (by weight) subset of signer set signed the command
/// batch
#[tokio::test]
async fn succeed_if_majority_of_subset_without_expected_weight_signed_batch() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], Some(150)).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let signing_signers = vec![signers[2].clone()]; // subset of the signer set
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signing_signers,
        &signers,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
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

    // Assert
    assert!(tx.result.is_ok());
}

#[tokio::test]
async fn fail_if_signed_commands_differ_from_the_execute_ones() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let messages_to_sign =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages_to_sign,
        &signers,
        &signers,
        quorum,
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("ProofError(LowSignaturesWeight)") }));
}

#[tokio::test]
async fn fail_if_quorum_differs_between_registered_and_signed() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages =
        [custom_message(destination_program_id, example_payload()).unwrap()].map(Either::Left);
    let (execute_data, gateway_execute_data_raw) = prepare_questionable_execute_data(
        &messages,
        &messages,
        &signers,
        &signers,
        quorum + 1, // quorum is different
        &gateway_root_pda,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
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

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("EpochNotFound") }));
}

/// fail if command len does not match provided account iter len
#[tokio::test]
async fn fail_if_command_len_does_not_match_provided_account_iter_len() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);
    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &execute_data.command_batch.commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            // we provide only 1 command pda, but there are 3 registered pdas
            &gateway_approved_command_pdas.as_slice()[..1],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        msg.contains("Mismatch between the number of commands and the number of accounts")
    }));
}

/// fail if command was not initialized
#[tokio::test]
async fn fail_if_command_was_not_initialised() {
    // Setup
    let (mut fixture, quorum, signers, gateway_root_pda) =
        setup_initialised_gateway(&[11, 22, 150], None).await;
    let destination_program_id = DestinationProgramId(Pubkey::new_unique());
    let messages = [
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
        custom_message(destination_program_id, example_payload()).unwrap(),
    ]
    .map(Either::Left);

    let (execute_data_pda, execute_data, ..) = fixture
        .init_execute_data(&gateway_root_pda, &messages, &signers, quorum)
        .await;

    // Action
    // none of the pdas are initialized
    let gateway_approved_command_pdas = execute_data
        .command_batch
        .commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(&gateway_root_pda, command);
            gateway_approved_message_pda
        })
        .collect_vec();
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        // note: error message is not very informative
        msg.contains("insufficient funds for instruction")
    }));
}
