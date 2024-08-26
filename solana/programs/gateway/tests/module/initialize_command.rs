use axelar_rkyv_encoding::types::Payload;
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::state::{
    ApprovedMessageStatus, GatewayApprovedCommand, GatewayCommandStatus, RotateSignersStatus,
};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::pubkey::Pubkey;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::test_setup::{
    make_signers, SigningVerifierSet, SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
    TestFixture,
};

use crate::{gateway_approved_command_ixs, make_payload_and_commands, program_test};

#[tokio::test]
async fn succesfully_initialize_validate_message_command() {
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
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    // Action
    let ixs = gateway_approved_command_ixs(&commands, gateway_root_pda, &fixture);
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
        assert!(!gateway_approved_command.is_validate_message_executed());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Pending)
        ));
    }
}

#[tokio::test]
async fn succesfully_initialize_rotate_signers_message() {
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

    // Signer set is slightly different to prevent hash collisions because there's
    // no random data
    let verifier_set = make_signers(&[44], 44);
    let payload = Payload::VerifierSet(verifier_set.verifier_set().clone());
    let command = OwnedCommand::RotateSigners(verifier_set.verifier_set());

    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    // Action
    let ixs = gateway_approved_command_ixs(&[command], gateway_root_pda, &fixture);
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
        assert!(!gateway_approved_command.is_validate_message_executed());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::RotateSigners(RotateSignersStatus::Pending)
        ));
    }
}

#[tokio::test]
async fn fail_when_gateway_root_pda_not_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let gateway_root_pda = Pubkey::new_unique();

    let (_, commands) = make_payload_and_commands(1);

    let ixs = gateway_approved_command_ixs(&commands, gateway_root_pda, &fixture)
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

    let (payload_1, _) = make_payload_and_commands(1);
    let (_execute_data_pubkey_1, _execute_data_1) = fixture
        .init_execute_data(&gateway_root_pda, payload_1, &signers, &domain_separator)
        .await;
    let (_payload_2, commands_2) = make_payload_and_commands(1);

    // Action
    let (pdas, ixs): (Vec<_>, Vec<_>) =
        gateway_approved_command_ixs(&commands_2, gateway_root_pda, &fixture)
            .into_iter()
            .unzip();
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
        assert!(!gateway_approved_command.is_validate_message_executed());
        assert!(matches!(
            gateway_approved_command.status(),
            GatewayCommandStatus::ApprovedMessage(ApprovedMessageStatus::Pending)
        ));
    }
}

#[tokio::test]
async fn fail_when_validate_message_already_initialized() {
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
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    let ixs = gateway_approved_command_ixs(&commands, gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
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
async fn fail_when_rotate_signers_is_already_initialized() {
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

    let new_signer_set = make_signers(&[44], 44);
    let payload = Payload::VerifierSet(new_signer_set.verifier_set().clone());
    let command = OwnedCommand::RotateSigners(new_signer_set.verifier_set());
    fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    let ixs: Vec<_> = gateway_approved_command_ixs(&[command], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect();
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
async fn succed_when_same_signers_with_diffrent_nonce_get_initialized() {
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

    // Signer set B is equal to A but with a different nonce.
    let signer_set_a = make_signers(&[10u128, 4], 10);
    let signer_set_b = SigningVerifierSet {
        nonce: 55,
        ..signer_set_a.clone()
    };

    // Payloads
    let payload_a = Payload::VerifierSet(signer_set_a.clone().verifier_set());
    let payload_b = Payload::VerifierSet(signer_set_b.clone().verifier_set());

    // Commands
    let command_a = OwnedCommand::RotateSigners(signer_set_a.verifier_set());
    let command_b = OwnedCommand::RotateSigners(signer_set_b.verifier_set());

    fixture
        .init_execute_data(&gateway_root_pda, payload_a, &signers, &domain_separator)
        .await;
    fixture
        .init_execute_data(&gateway_root_pda, payload_b, &signers, &domain_separator)
        .await;
    let ixs_a = gateway_approved_command_ixs(&[command_a], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    fixture.send_tx(&ixs_a).await;

    // Action
    let ixs_b = gateway_approved_command_ixs(&[command_b], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    let BanksTransactionResultWithMetadata {
        metadata: _,
        result,
    } = fixture.send_tx_with_metadata(&ixs_b).await;

    // Assert
    assert!(result.is_ok(), "Transaction should not have failed");
}
