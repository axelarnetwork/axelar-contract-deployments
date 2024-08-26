use axelar_rkyv_encoding::types::{Payload, VerifierSet};
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::instructions::GatewayInstruction;
use gmp_gateway::state::GatewayConfig;
use solana_program_test::tokio;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::{
    make_signers, SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};

use crate::{
    get_approved_command, get_gateway_events, get_gateway_events_from_execute_data, make_messages,
    make_payload_and_commands,
};

fn payload_and_command(verifier_set: &VerifierSet) -> (Payload, [OwnedCommand; 1]) {
    let payload = Payload::VerifierSet(verifier_set.clone());
    let command = OwnedCommand::RotateSigners(verifier_set.clone());
    (payload, [command])
}

/// successfully process execute when there is 1 rotate signers commands
#[ignore]
#[tokio::test]
async fn successfully_rotates_signers() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .build()
        .setup()
        .await;
    let new_signer_set = make_signers(&[500, 200], 1);
    let (payload, command) = payload_and_command(&new_signer_set.verifier_set());

    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;
    let gateway_approved_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let tx = fixture
        .rotate_signers_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pda,
            &signers.verifier_set_tracker(),
            &new_signer_set.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    // - expected events
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs = get_gateway_events_from_execute_data(&command);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    let approved_command = get_approved_command(&mut fixture, &gateway_approved_command_pda).await;
    assert!(approved_command.is_command_executed());

    // - signers have been updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = 2u128;
    assert_eq!(
        root_pda_data.auth_weighted.current_epoch(),
        new_epoch.into()
    );
    // todo -- assert that the signer tracker pda has been initialized

    // - test that both signer sets can sign new messages
    for signer_set in [new_signer_set, signers] {
        let messages = make_messages(1);
        fixture
            .fully_approve_messages(&gateway_root_pda, messages, &signer_set, &domain_separator)
            .await;
    }
}

#[tokio::test]
async fn cannot_invoke_rotate_signers_without_respecting_minimum_delay() {
    // Setup
    let minimum_delay_seconds = 3;
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 42, 33])
        .minimum_rotate_signers_delay_seconds(minimum_delay_seconds)
        .build()
        .setup()
        .await;
    // after we set up the gateway, the minimumn delay needs to be forwarded
    fixture.forward_time(minimum_delay_seconds as i64).await;

    // Action - rotate the signer set for the first time.
    let new_signer_set = make_signers(&[500, 200], 1);
    fixture
        .fully_rotate_signers(
            &gateway_root_pda,
            new_signer_set.verifier_set(),
            &signers,
            &domain_separator,
        )
        .await;

    // Action - rotate the signer set for the second time. As this action succeeds
    // without waiting the minimum_delay_seconds, it should fail.
    let newer_signer_set = make_signers(&[444, 555], 333);
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            newer_signer_set.verifier_set(),
            &new_signer_set,
            &domain_separator,
        )
        .await;

    // Assert we are seeing the correct error message in tx logs.
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Command needs more time before being executed again",) }));

    // Action, forward time
    fixture.forward_time(minimum_delay_seconds as i64).await;

    // Action, rotate signers again after waiting the minimum delay.
    let newer_signer_set = make_signers(&[444, 555], 333);
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            newer_signer_set.verifier_set(),
            &new_signer_set,
            &domain_separator,
        )
        .await;
    // Assert the rotate_signers transaction succeeded after waiting the time bound
    // checks required delay.
    assert!(tx.result.is_ok())
}

/// Ensure that we can use an old signer set to sign messages as long as the
/// operator also signed the `rotate_signers` ix
#[ignore]
#[tokio::test]
async fn succeed_if_signer_set_signed_by_old_signer_set_and_submitted_by_the_operator() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        operator,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;
    // -- we set a new signer set to be the "latest" signer set
    let new_signer_set = make_signers(&[500, 200], 1);
    fixture
        .fully_rotate_signers(
            &gateway_root_pda,
            new_signer_set.verifier_set(),
            &signers,
            &domain_separator,
        )
        .await;

    let newer_signer_set = make_signers(&[500, 200], 2);
    let (payload, command) = payload_and_command(&new_signer_set.verifier_set());
    // we stil use the initial signer set to sign the data (the `signers` variable)
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let ix = gmp_gateway::instructions::rotate_signers(
        execute_data_pda,
        gateway_root_pda,
        rotate_signers_command_pda,
        Some(operator.pubkey()),
        signers.verifier_set_tracker(),
        newer_signer_set.verifier_set_tracker(),
        fixture.payer.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&operator, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let emitted_events = get_gateway_events(&tx);
    let expected_approved_command_logs = get_gateway_events_from_execute_data(&command);
    for (actual, expected) in emitted_events
        .iter()
        .zip(expected_approved_command_logs.iter())
    {
        assert_eq!(actual, expected);
    }

    // - command PDAs get updated
    let approved_command = get_approved_command(&mut fixture, &rotate_signers_command_pda).await;
    assert!(approved_command.is_command_executed());

    // - signers have been updated
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let new_epoch = 3u128;
    assert_eq!(
        root_pda_data.auth_weighted.current_epoch(),
        new_epoch.into()
    );
    // todo -- assert verifier set pda
}

/// We use a different account in place of the expected operator to try and
/// rotate signers - but an on-chain check rejects his attempts
#[ignore]
#[tokio::test]
async fn fail_if_provided_operator_is_not_the_real_operator_thats_stored_in_gateway_state() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;
    // -- we set a new signer set to be the "latest" signer set
    let new_signer_set = make_signers(&[500, 200], 1);
    fixture
        .fully_rotate_signers(
            &gateway_root_pda,
            new_signer_set.verifier_set(),
            &signers,
            &domain_separator,
        )
        .await;

    let newer_signer_set = make_signers(&[500, 200], 700);
    let (payload, command) = payload_and_command(&newer_signer_set.verifier_set());

    // we stil use the initial signer set to sign the data (the `signers` variable)
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let fake_operator = Keypair::new();
    let ix = gmp_gateway::instructions::rotate_signers(
        execute_data_pda,
        gateway_root_pda,
        rotate_signers_command_pda,
        Some(fake_operator.pubkey()), // `stranger_danger` in place of the expected `operator`
        signers.verifier_set_tracker(),
        newer_signer_set.verifier_set_tracker(),
        fixture.payer.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&fake_operator, &fixture.payer.insecure_clone()],
        )
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}

/// ensure that the operator still needs to use a valid signer set to to
/// force-rotate the signers
#[ignore]
#[tokio::test]
async fn fail_if_operator_is_not_using_pre_registered_signer_set() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        gateway_root_pda,
        operator,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;
    // generate a new random operator set to be used (do not register it)
    let new_signer_set = make_signers(&[500, 200], 1);
    let random_signer_set = make_signers(&[11], 54);
    let (payload, command) = payload_and_command(&new_signer_set.verifier_set());

    // using `new_signers` which is the cause of the failure
    let (execute_data_pda, _) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &random_signer_set,
            &domain_separator,
        )
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let ix = gmp_gateway::instructions::rotate_signers(
        execute_data_pda,
        gateway_root_pda,
        rotate_signers_command_pda,
        Some(operator.pubkey()),
        random_signer_set.verifier_set_tracker(),
        new_signer_set.verifier_set_tracker(),
        fixture.payer.pubkey(),
    )
    .unwrap();
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(
            &[ix],
            &[&operator, &fixture.payer.insecure_clone()],
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

/// Ensure that the operator also need to explicitly sign the ix
#[ignore]
#[tokio::test]
async fn fail_if_operator_only_passed_but_not_actual_signer() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        operator,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;
    // -- we set a new signer set to be the "latest" signer set
    let new_signer_set = make_signers(&[500, 200], 1);
    let (payload, command) = payload_and_command(&new_signer_set.verifier_set());

    // we stil use the initial signer set to sign the data (the `signers` variable)
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;
    let rotate_signers_command_pda = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();

    // Action
    let data = borsh::to_vec(&GatewayInstruction::RotateSigners).unwrap();
    let accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new(execute_data_pda, false),
        AccountMeta::new(rotate_signers_command_pda, false),
        AccountMeta::new(operator.pubkey(), false), /* the flag being `false` is the cause of
                                                     * failure for the tx */
        AccountMeta::new_readonly(signers.verifier_set_tracker(), false),
        AccountMeta::new(new_signer_set.verifier_set_tracker(), false),
        AccountMeta::new(fixture.payer.pubkey(), true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let ix = Instruction {
        program_id: gmp_gateway::id(),
        accounts,
        data,
    };
    let tx = fixture
        .send_tx_with_custom_signers_with_metadata(&[ix], &[&fixture.payer.insecure_clone()])
        .await;

    // Assert
    assert!(tx.result.is_err());
    assert!(tx
        .metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|msg| { msg.contains("Proof is not signed by the latest signer set") }));
}
/// disallow rotate signers if any other signer set besides the most recent
/// epoch signed the proof
#[ignore]
#[tokio::test]
async fn fail_if_rotate_signers_signed_by_old_signer_set() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;
    let new_signer_set = make_signers(&[500, 200], 1);
    fixture
        .fully_rotate_signers(
            &gateway_root_pda,
            new_signer_set.verifier_set(),
            &signers,
            &domain_separator,
        )
        .await;

    // Action
    let newer_signer_set = make_signers(&[444, 555], 333);
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            newer_signer_set.verifier_set(),
            &signers,
            &domain_separator,
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

/// `rotate_signer_set` is ignored if total weight is smaller than new
/// command weight quorum (tx succeeds)
#[ignore]
#[tokio::test]
async fn ignore_rotate_signers_if_total_weight_is_smaller_than_quorum() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;
    let new_signer_set = make_signers(&[1, 1], 10);

    // Action
    let (.., tx) = fixture
        .fully_rotate_signers_with_execute_metadata(
            &gateway_root_pda,
            new_signer_set.verifier_set(),
            &signers,
            &domain_separator,
        )
        .await;

    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = 1u128;
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch.into());
    // todo -- assert verifier set pda
}

#[ignore]
#[tokio::test]
async fn fail_if_order_of_commands_is_not_the_same_as_order_of_accounts() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;

    let (payload, commands) = make_payload_and_commands(3);

    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    // Action
    let mut gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &commands)
        .await;
    gateway_approved_command_pdas.reverse();

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
}

/// `rotate_signer_set` is ignored if new signer set len is 0 (tx succeeds)
#[ignore]
#[tokio::test]
async fn fail_on_rotate_signers_if_new_ops_len_is_zero() {
    // Setup
    let SolanaAxelarIntegrationMetadata {
        mut fixture,
        signers,
        gateway_root_pda,
        domain_separator,
        ..
    } = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![11, 22, 150])
        .build()
        .setup()
        .await;

    let new_signer_set = make_signers(&[], 1);
    let (payload, command) = payload_and_command(&new_signer_set.verifier_set());
    let (execute_data_pda, _) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    // Action
    let gateway_approved_command_pdas = fixture
        .init_pending_gateway_commands(&gateway_root_pda, &command)
        .await
        .pop()
        .unwrap();
    let tx = fixture
        .rotate_signers_with_metadata(
            &gateway_root_pda,
            &execute_data_pda,
            &gateway_approved_command_pdas,
            &signers.verifier_set_tracker(),
            &new_signer_set.verifier_set_tracker(),
        )
        .await;

    // Assert
    assert!(tx.result.is_ok());
    let gateway = fixture
        .get_account::<GatewayConfig>(&gateway_root_pda, &gmp_gateway::ID)
        .await;
    let constant_epoch = 1u128;
    assert_eq!(gateway.auth_weighted.current_epoch(), constant_epoch.into());
    // todo -- assert verifier set pda
}
