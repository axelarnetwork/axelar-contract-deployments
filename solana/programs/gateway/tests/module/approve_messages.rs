use axelar_message_primitives::command::U256;
use axelar_rkyv_encoding::types::{ArchivedExecuteData, ExecuteData, Payload};
use gmp_gateway::state::GatewayApprovedCommand;
use itertools::Itertools;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use test_fixtures::test_setup::{
    make_signers, SigningVerifierSet, SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};

use crate::{
    get_approved_command, get_gateway_events, get_gateway_events_from_execute_data, make_message,
    make_messages, make_payload_and_commands, payload_and_commands,
    prepare_questionable_execute_data,
};

#[tokio::test]
async fn successfully_approves_commands_when_there_are_no_commands() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;
    let messages = Payload::new_messages(vec![]);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, messages, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_ok())
}

/// successfully approves messages when there are 3 validate message
/// commands - emits message approved events
#[tokio::test]
async fn successfully_approves_commands_when_there_are_3_validate_message_commands() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let (payload, commands) = make_payload_and_commands(3);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
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
#[tokio::test]
async fn successfully_consumes_repeating_commands_idempotency_same_batch() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let (payload, commands) = make_payload_and_commands(1);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
            &signers.verifier_set_tracker(),
        )
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
            &signers.verifier_set_tracker(),
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
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![10, 4])
        .build()
        .setup()
        .await;

    let messages = make_messages(1);
    let (payload, commands) = payload_and_commands(&messages);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
        .await;
    fixture
        .approve_pending_gateway_messages(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
            &signers.verifier_set_tracker(),
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
        .init_execute_data(&gateway_root_pda, new_payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
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
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers: _,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder().build().setup().await;

    let (payload, commands) = make_payload_and_commands(1);

    let signers = make_signers(&[11, 22], 11);

    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Invalid VerifierSetTracker PDA") }));
}

/// fail if if root config has no signers and there are no signatures in the
/// execute data
#[tokio::test]
async fn fail_if_gateway_config_has_no_signers_signed_by_empty_set() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder().build().setup().await;

    let (payload, commands) = make_payload_and_commands(1);
    let (execute_data_pda, raw_execute_data) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    let archived_execute_data = ArchivedExecuteData::from_bytes(&raw_execute_data).unwrap();
    assert!(archived_execute_data
        .proof()
        .signers_with_signatures()
        .is_empty());

    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
        .await;

    // Action
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("MessageValidationError(InsufficientWeight)") }));
}

/// fail if root config not initialised
#[tokio::test]
async fn fail_if_root_config_not_initialised() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(0);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
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
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .build()
        .setup()
        .await;

    let (payload, commands) = make_payload_and_commands(0);
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
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
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(3);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
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
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(1);
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Failed to deserialize execute_data bytes") }));
}

/// fail if epoch for signers was not found (inside `validate_proof`)
#[tokio::test]
async fn fail_if_epoch_for_signers_was_not_found() {
    // Setup
    let unregistered_signer_set_signers = make_signers(&[55u128, 66], 10);
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers: _signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(1);
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &unregistered_signer_set_signers,
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
            &unregistered_signer_set_signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Invalid VerifierSetTracker PDA") }));
}

/// fail if signer set epoch is older than 4 epochs away (inside
/// `validate_proof`)
#[tokio::test]
async fn fail_if_signer_set_epoch_is_older_than_4() {
    // Setup
    const MAX_ALLOWED_SIGNERS: usize = 4;
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers: initial_signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .previous_signers_retention(MAX_ALLOWED_SIGNERS as u64)
        .minimum_rotate_signers_delay_seconds(0)
        .build()
        .setup()
        .await;

    // We generate 4 new unique signer sets (not registered yet)
    let new_signer_sets = (1..=MAX_ALLOWED_SIGNERS as u128)
        .map(|weight| make_signers(&[55u128, weight], 55 + weight as u64))
        .collect::<Vec<_>>();

    // Only the latest signer set is allowed to call "rotate signers" ix
    // to register the next latest signer set. We iterate over all signer sets,
    // calling "rotate signer" with the last known signer set to register
    // the next latest signer set.
    dbg!("rotating singers");
    for (idx, (current_signers, new_signers)) in
        ([&initial_signers].into_iter().chain(new_signer_sets.iter()))
            .tuple_windows::<(_, _)>()
            .enumerate()
    {
        dbg!("rotate idx", &idx);
        let new_epoch = U256::from((idx + 1) as u128);
        let root_pda_data = fixture
            .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
            .await;
        assert_eq!(root_pda_data.auth_weighted.current_epoch(), new_epoch);

        fixture
            .fully_rotate_signers(
                &gateway_root_pda,
                new_signers.verifier_set(),
                current_signers,
                &domain_separator,
            )
            .await;
    }
    dbg!("signers rotated");

    // Now we have registered 5 sets in total (1 initial signer set + 4 that we
    // generated). The "epoch" is an incremental counter. But the data structure
    // only kept around 4 entries.
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let current_epoch = U256::from(MAX_ALLOWED_SIGNERS + 1);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);

    // Action
    // Any of the lastest 4 signer sets are allowed to "approve messages" coming
    // Axelar->Solana direction.
    for signer_set in &new_signer_sets {
        fixture
            .fully_approve_messages(
                &gateway_root_pda,
                make_messages(1),
                signer_set,
                &domain_separator,
            )
            .await;
    }

    // We cannot use the first signer set anymore.
    let (.., tx) = fixture
        .fully_approve_messages_with_execute_metadata(
            &gateway_root_pda,
            make_messages(1),
            &initial_signers,
            &domain_separator,
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("verifier set is too old") }));
}

/// fail if signatures cannot be recovered (inside `validate_signatures`
/// ProofError::Secp256k1RecoverError)
#[tokio::test]
async fn fail_if_invalid_signatures() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers: registered_signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(1);

    // Action
    // -----
    // We generate a new *valid* execute data bytes, then we malform a single
    // signature to become unrecoverable
    let execute_data_raw_bytes = prepare_questionable_execute_data(
        &payload,
        &payload,
        &registered_signers,
        &registered_signers,
        &domain_separator,
    );
    let mut ex = ExecuteData::from_bytes(&execute_data_raw_bytes).unwrap();
    if let Some(x) = ex
        .proof
        .signers_with_signatures
        .mut_inner_map()
        .iter_mut()
        .next()
        .unwrap()
        .1
        .signature
        .as_mut()
    {
        // flip all bits of every byte
        for byte in x.as_mut() {
            *byte = !*byte;
        }
    }
    let execute_data_raw_bytes = ex
        .to_bytes::<0>()
        .expect("failed to serialize 'ExecuteData' struct");
    // Signature malformation finished
    // ------
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &execute_data_raw_bytes,
            &domain_separator,
        )
        .await;
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
        .await;
    let tx = fixture
        .approve_pending_gateway_messages_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
            &registered_signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Failed to recover ECDSA signature") }));
}

/// fail if invalid signer set signed the command batch
#[tokio::test]
async fn fail_if_invalid_signer_set_signed_command_batch() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers: registered_signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11])
        .build()
        .setup()
        .await;

    let unregistered_signers = make_signers(&[11], 321);

    let (payload, commands) = make_payload_and_commands(1);
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &unregistered_signers,
        &registered_signers,
        &domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
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
            &registered_signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("InvalidSignerSet") }));
}

/// fail if small subset signers signed the command batch (inside
/// `validate_signatures` ProofError::LowSignatureWeight)
#[tokio::test]
async fn fail_if_subset_without_expected_weight_signed_batch() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 15, 150])
        .build()
        .setup()
        .await;

    let signing_signers = signers.signers.iter().take(2).cloned().collect::<Vec<_>>(); // subset of the signer set
    let signing_signers = SigningVerifierSet {
        signers: signing_signers,
        ..signers.clone()
    };

    let (payload, commands) = make_payload_and_commands(1);

    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &signing_signers,
        &signers,
        &domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("MessageValidationError(InsufficientWeight)") }));
}

/// succeed if the larger (by weight) subset of signer set signed the command
/// batch
#[tokio::test]
async fn succeed_if_majority_of_subset_without_expected_weight_signed_batch() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .custom_quorum(150)
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(1);
    let signing_signers = [signers.signers[2].clone()]; // subset of the signer set
    let signing_signers = SigningVerifierSet {
        signers: signing_signers.to_vec(),
        ..signers.clone()
    };
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &signing_signers,
        &signers,
        &domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
}

#[tokio::test]
async fn fail_if_signed_commands_differ_from_the_execute_ones() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 15, 150])
        .build()
        .setup()
        .await;

    let different_messages = vec![make_message()];
    let (payload, commands) = make_payload_and_commands(1);
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        // messages that get included in the hash that gets signed are different from the messages
        // that will be included in the execute data
        &Payload::new_messages(different_messages),
        &payload,
        &signers,
        &signers,
        &domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("MessageValidationError(InvalidSignature)") }));
}

#[tokio::test]
async fn fail_if_quorum_differs_between_registered_and_signed() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 15, 150])
        .build()
        .setup()
        .await;

    let altered_signers = SigningVerifierSet {
        quorum: (u128::from(signers.quorum) + 1_u128).into(),
        ..signers.clone()
    };
    let (payload, commands) = make_payload_and_commands(1);
    let gateway_execute_data_raw = prepare_questionable_execute_data(
        &payload,
        &payload,
        &signers,
        &altered_signers,
        &domain_separator,
    );
    let execute_data_pda = fixture
        .init_execute_data_with_custom_data(
            &gateway_root_pda,
            &gateway_execute_data_raw,
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("InvalidSignerSet") }));
}

/// fail if command len does not match provided account iter len[
#[tokio::test]
async fn fail_if_command_len_does_not_match_provided_account_iter_len() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 15, 150])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(3);
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
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
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 15, 150])
        .build()
        .setup()
        .await;
    let (payload, commands) = make_payload_and_commands(3);

    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
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
            &signers.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx.metadata.unwrap().log_messages.into_iter().any(|msg| {
        // note: error message is not very informative
        msg.contains("insufficient funds for instruction")
    }));
}
