use axelar_rkyv_encoding::types::Payload;
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::state::{
    ApprovedMessageStatus, GatewayApprovedCommand, GatewayCommandStatus, RotateSignersStatus,
};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::pubkey::Pubkey;
use test_fixtures::account::CheckValidPDAInTests;
use test_fixtures::execute_data::prepare_execute_data;
use test_fixtures::test_setup::TestFixture;
use test_fixtures::test_signer::create_signer_with_weight;

use crate::{
    create_verifier_set_with_nonce, example_signer_set, gateway_approved_command_ixs,
    make_payload_and_commands, make_signers, program_test,
};

const NONCE: u64 = 44;

#[ignore]
#[tokio::test]
async fn succesfully_initialize_validate_message_command() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers, NONCE),
            Pubkey::new_unique(),
        )
        .await;

    let (payload, commands) = make_payload_and_commands(3);
    let domain_separator = fixture.domain_separator;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
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

#[ignore]
#[tokio::test]
async fn succesfully_initialize_rotate_signers_message() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];

    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers, NONCE),
            Pubkey::new_unique(),
        )
        .await;

    // Signer set is slightly different to prevent hash collisions because there's
    // no random data
    let verifier_set = example_signer_set(44, 44);
    let payload = Payload::VerifierSet(verifier_set.clone());
    let command = OwnedCommand::RotateSigners(verifier_set);

    let domain_separator = fixture.domain_separator;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
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

#[ignore]
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

#[ignore]
#[tokio::test]
async fn succesfully_initialize_command_which_belongs_to_a_different_execute_data_set() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers, NONCE),
            Pubkey::new_unique(),
        )
        .await;
    let (payload_1, _) = make_payload_and_commands(1);
    let domain_separator = fixture.domain_separator;
    let (_execute_data_pubkey_1, _execute_data_1) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload_1,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
        .await;
    let (payload_2, commands_2) = make_payload_and_commands(1);
    prepare_execute_data(payload_2, &signers, quorum, NONCE, &domain_separator); // todo remove this?

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

#[ignore]
#[tokio::test]
async fn fail_when_validate_message_already_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers, NONCE),
            Pubkey::new_unique(),
        )
        .await;
    let (payload, commands) = make_payload_and_commands(1);
    let domain_separator = fixture.domain_separator;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
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

#[ignore]
#[tokio::test]
async fn fail_when_rotate_signers_is_already_initialized() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers, NONCE),
            Pubkey::new_unique(),
        )
        .await;
    let new_signer_set = example_signer_set(42, 43);
    let payload = Payload::VerifierSet(new_signer_set.clone());
    let command = OwnedCommand::RotateSigners(new_signer_set);
    let domain_separator = fixture.domain_separator;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
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
#[ignore]
#[tokio::test]
async fn fail_when_rotate_signers_has_unchanged_block_height() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = make_signers(&[10_u128, 4_u128]);
    let quorum = 14;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers, NONCE),
            Pubkey::new_unique(),
        )
        .await;
    let domain_separator = fixture.domain_separator;

    // Signer set B is equal to A but with a different nonce.
    let new_signers = make_signers(&[10u128, 4]);
    let signer_set_a = create_verifier_set_with_nonce(&new_signers, 180, 111);
    let signer_set_b = create_verifier_set_with_nonce(&new_signers, 360, 111);

    // Payloads
    let payload_a = Payload::VerifierSet(signer_set_a.clone());
    let payload_b = Payload::VerifierSet(signer_set_b.clone());

    // Commands
    let command_a = OwnedCommand::RotateSigners(signer_set_a);
    let command_b = OwnedCommand::RotateSigners(signer_set_b);

    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload_a,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
        .await;
    fixture
        .init_execute_data(
            &gateway_root_pda,
            payload_b,
            &signers,
            quorum,
            NONCE,
            &domain_separator,
        )
        .await;
    let ixs_a = gateway_approved_command_ixs(&[command_a], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    fixture.send_tx(&ixs_a).await;

    // Action -- will fail because the `created_at` field gets dropped when encoded
    // resulting in the same hash for the command
    let ixs_b = gateway_approved_command_ixs(&[command_b], gateway_root_pda, &fixture)
        .into_iter()
        .map(|(_, ix)| ix)
        .collect::<Vec<_>>();
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&ixs_b).await;

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
