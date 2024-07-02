use axelar_message_primitives::command::U256;
use axelar_rkyv_encoding::types::{ArchivedExecuteData, Payload, VerifierSet};
use gmp_gateway::state::GatewayApprovedCommand;
use itertools::Itertools;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use test_fixtures::axelar_message::new_signer_set;
use test_fixtures::test_signer::TestSigner;

use crate::{
    create_signer_set, get_approved_command, get_gateway_events,
    get_gateway_events_from_execute_data, make_message, make_messages, make_payload_and_commands,
    make_signers, payload_and_commands, prepare_questionable_execute_data,
    setup_initialised_gateway, InitialisedGatewayMetadata,
};

#[ignore]
#[tokio::test]
async fn successfully_process_execute_when_there_are_no_commands() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let messages = Payload::Messages(vec![]);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            messages,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;

    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &[])
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
#[ignore]
#[tokio::test]
async fn successfully_process_execute_when_there_are_3_validate_message_commands() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;

    let (payload, commands) = make_payload_and_commands(3);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;

    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
    let expected_approved_command_logs = get_gateway_events_from_execute_data(&commands);
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

/// calling the same execute flow multiple times with the same execute data will
/// not "approve" the command twice.
#[ignore]
#[tokio::test]
async fn successfully_consumes_repeating_commands_idempotency_same_batch() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let (payload, commands) = make_payload_and_commands(1);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn successfully_consumes_repeating_commands_idempotency_unique_batches() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 42, 33], None).await;
    let domain_separator = fixture.domain_separator;
    let messages = make_messages(1);
    let (payload, commands) = payload_and_commands(&messages);
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
    let mut new_messages = messages.clone();
    new_messages.push(make_message());
    let (new_payload, new_commands) = payload_and_commands(&new_messages);

    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            new_payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pda_new = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &[new_commands[1].clone()])
        .await[0];

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
#[ignore]
#[tokio::test]
async fn fail_if_gateway_config_has_no_signers_signed_by_unknown_signer_set() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[], None).await;
    let (payload, commands) = make_payload_and_commands(1);

    let signers = make_signers(&[11, 22]);

    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_gateway_config_has_no_signers_signed_by_empty_set() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[], None).await;
    let (payload, commands) = make_payload_and_commands(1);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, raw_execute_data) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;

    let archived_execute_data = ArchivedExecuteData::from_bytes(&raw_execute_data).unwrap();
    assert!(archived_execute_data.proof().signatures().is_empty());

    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_root_config_not_initialised() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;
    let (payload, commands) = make_payload_and_commands(0);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_execute_data_not_initialised() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;
    let (payload, commands) = make_payload_and_commands(0);
    let domain_separator = fixture.domain_separator;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_invalid_account_for_gateway() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;
    let (payload, commands) = make_payload_and_commands(3);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_invalid_account_for_execute_data() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;
    let (payload, commands) = make_payload_and_commands(1);
    let domain_separator = fixture.domain_separator;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_epoch_for_signers_was_not_found() {
    // Setup
    let (_unregistered_signer_set, unregistered_signer_set_signers) =
        create_signer_set(&[55u128, 66], 10u128);
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;
    let (payload, commands) = make_payload_and_commands(1);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &unregistered_signer_set_signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_signer_set_epoch_is_older_than_16() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        signers: initial_signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;

    let initial_signer_set: [(VerifierSet, Vec<TestSigner>); 1] = [(
        new_signer_set(&initial_signers, 0, 33),
        initial_signers.clone(),
    )];

    // We generate 4 new unique signer sets (not registered yet)
    let new_signer_sets = (1..=4)
        .map(|weight| create_signer_set(&[55u128, weight], 55u128 + weight))
        .collect::<Vec<_>>();

    // Only the latest signer set is allowed to call "rotate signers" ix
    // to register the next latest signer set. We iterate over all signer sets,
    // calling "rotate signer" with the last known signer set to register
    // the next latest signer set.
    for (idx, ((_, current_signer_set_signers), (new_signer_set, _))) in
        (initial_signer_set.iter().chain(new_signer_sets.iter()))
            .tuple_windows::<(_, _)>()
            .enumerate()
    {
        let new_epoch = U256::from((idx + 1) as u128);
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

    // Now we have registered 5 sets in total (1 initial signer set + 4 that we
    // generated). The "epoch" is an incremental counter. But the data structure
    // only kept around 4 entries.
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = U256::from(5u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);
    assert_eq!(root_pda_data.auth_weighted.signer_sets().len(), 4);

    // Action
    // Any of the lastest 4 signer sets are allowed to "approve messages" coming
    // Axelar->Solana direction.
    for (_, signer_set) in &new_signer_sets {
        fixture
            .fully_approve_messages(&gateway_root_pda, make_messages(1), signer_set)
            .await;
    }

    // We cannot use the first signer set anymore.
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(
            &gateway_root_pda,
            make_messages(1),
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
#[ignore]
#[tokio::test]
async fn fail_if_invalid_signatures() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum: threshold,
        gateway_root_pda,
        signers: registered_signers,
        ..
    } = setup_initialised_gateway(&[11, 22], None).await;

    let unregistered_signers = make_signers(&[11, 22]);

    let (payload, commands) = make_payload_and_commands(1);

    let execute_data_raw_bytes = prepare_questionable_execute_data(
        &payload,
        &payload,
        &unregistered_signers,
        &registered_signers,
        threshold,
        &fixture.domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &execute_data_raw_bytes)
        .await;

    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_invalid_signer_set_signed_command_batch() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers: registered_signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11], None).await;

    let unregistered_signers = make_signers(&[11]);

    let (payload, commands) = make_payload_and_commands(1);
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &unregistered_signers,
        &registered_signers,
        quorum,
        &fixture.domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &gateway_execute_data_raw)
        .await;

    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_subset_without_expected_weight_signed_batch() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;

    let signing_signers = signers.iter().take(2).cloned().collect::<Vec<_>>(); // subset of the signer set

    let (payload, commands) = make_payload_and_commands(1);

    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &signing_signers,
        &signers,
        quorum,
        &fixture.domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &gateway_execute_data_raw)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn succeed_if_majority_of_subset_without_expected_weight_signed_batch() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], Some(150)).await;
    let (payload, commands) = make_payload_and_commands(1);
    let signing_signers = [signers[2].clone()]; // subset of the signer set
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &signing_signers,
        &signers,
        quorum,
        &fixture.domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &gateway_execute_data_raw)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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
#[ignore]
#[tokio::test]
async fn fail_if_signed_commands_differ_from_the_execute_ones() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;

    let messages = vec![make_message()];
    let (payload, commands) = make_payload_and_commands(1);
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &Payload::Messages(messages),
        &payload,
        &signers,
        &signers,
        quorum,
        &fixture.domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &gateway_execute_data_raw)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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

#[ignore]
#[tokio::test]
async fn fail_if_quorum_differs_between_registered_and_signed() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;

    let (payload, commands) = make_payload_and_commands(1);
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &signers,
        &signers,
        quorum + 1, // quorum is different
        &fixture.domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(&gateway_root_pda, &gateway_execute_data_raw)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
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

/// fail if command len does not match provided account iter len[
#[ignore]
#[tokio::test]
async fn fail_if_command_len_does_not_match_provided_account_iter_len() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    let (payload, commands) = make_payload_and_commands(3);
    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            // we provide only 1 command pda, but there are 3 registered pdas
            &gateway_approved_command_pdas[..1],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        msg.contains("Mismatch between the number of commands and the number of accounts")
    }));
}

/// fail if command was not initialized
#[ignore]
#[tokio::test]
async fn fail_if_command_was_not_initialised() {
    // Setup
    let InitialisedGatewayMetadata {
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[11, 22, 150], None).await;
    let (payload, commands) = make_payload_and_commands(3);

    let domain_separator = fixture.domain_separator;
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            &domain_separator,
        )
        .await;

    // Action
    // none of the pdas are initialized
    let gateway_approved_command_pdas = commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(&gateway_root_pda, command);
            gateway_approved_message_pda
        })
        .collect::<Vec<_>>();

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
