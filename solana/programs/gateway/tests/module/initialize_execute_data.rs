use axelar_rkyv_encoding::types::{HasheableMessageVec, Payload, VerifierSet};
use gmp_gateway::instructions::InitializeConfig;
use gmp_gateway::state::execute_data::ArchivedGatewayExecuteData;
use gmp_gateway::state::{GatewayConfig, GatewayExecuteData};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::system_program;
use test_fixtures::execute_data::prepare_execute_data;
use test_fixtures::test_setup::{
    make_signers, make_signers_with_quorum, SolanaAxelarIntegration,
    SolanaAxelarIntegrationMetadata, TestFixture,
};

use crate::{make_messages, make_payload_and_commands, program_test};

#[tokio::test]
async fn test_successfylly_initialize_execute_data() {
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

    let payload = Payload::new_messages(make_messages(1));
    let (raw_execute_data, _) = prepare_execute_data(payload, &signers, &domain_separator);
    let gateway_execute_data = GatewayExecuteData::<HasheableMessageVec>::new(
        &raw_execute_data,
        &gateway_root_pda,
        &domain_separator,
    )
    .expect("valid GatewayExecuteData");

    let (execute_data_pda, _) = gmp_gateway::get_execute_data_pda(
        &gateway_root_pda,
        &gateway_execute_data.hash_decoded_contents(),
    );

    // Action
    fixture
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            gmp_gateway::instructions::initialize_approve_messages_execute_data(
                fixture.payer.pubkey(),
                gateway_root_pda,
                &domain_separator,
                &raw_execute_data,
            )
            .unwrap()
            .0,
        ])
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
        ArchivedGatewayExecuteData::<HasheableMessageVec>::from_bytes(account.data.as_slice())
            .expect("GatewayExecuteData can be deserialized");
    assert_eq!(*deserialized_gateway_execute_data, gateway_execute_data);
}

#[tokio::test]
async fn test_succesfully_initialize_rotate_signers() {
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
    let nonce = 55;
    let new_signers = make_signers(&[33, 150], nonce);

    let payload = Payload::VerifierSet(new_signers.verifier_set());

    let (raw_execute_data, _) = prepare_execute_data(payload, &signers, &domain_separator);
    let gateway_execute_data = GatewayExecuteData::<VerifierSet>::new(
        &raw_execute_data,
        &gateway_root_pda,
        &domain_separator,
    )
    .expect("valid GatewayExecuteData");
    let (execute_data_pda, _) = gmp_gateway::get_execute_data_pda(
        &gateway_root_pda,
        &gateway_execute_data.hash_decoded_contents(),
    );

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_rotate_signers_execute_data(
        fixture.payer.pubkey(),
        gateway_root_pda,
        &domain_separator,
        &raw_execute_data,
    )
    .expect("failed to create initialize_execute_data instruction");

    fixture
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            ix,
        ])
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
        ArchivedGatewayExecuteData::<VerifierSet>::from_bytes(account.data.as_slice())
            .expect("GatewayExecuteData can be deserialized");
    assert_eq!(*deserialized_gateway_execute_data, gateway_execute_data);
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
    let nonce = 123;
    let quorum = 14;
    let signers = make_signers_with_quorum(&[10, 4], nonce, quorum);
    let domain_separator = [255; 32];
    fixture
        .initialize_gateway_config_account(InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[&signers]),
            ..fixture.base_initialize_config(domain_separator)
        })
        .await;
    let (payload, _) = make_payload_and_commands(1);
    let (raw_execute_data, _) = prepare_execute_data(payload, &signers, &domain_separator);

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_approve_messages_execute_data(
        fixture.payer.pubkey(),
        fake_gateway_root_pda,
        &domain_separator,
        &raw_execute_data,
    )
    .expect("failed to create initialize_execute_data instruction");

    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&[ix]).await;

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
    let quorum = 14;
    let nonce = 123321;
    let domain_separator = [32; 32];
    let signers = make_signers_with_quorum(&[10, 4], nonce, quorum);
    fixture
        .initialize_gateway_config_account(InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[&signers]),
            ..fixture.base_initialize_config(domain_separator)
        })
        .await;
    let (payload, _) = make_payload_and_commands(1);
    let (raw_execute_data, _) = prepare_execute_data(payload, &signers, &domain_separator);

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_approve_messages_execute_data(
        fixture.payer.pubkey(),
        fake_gateway_root_pda,
        // gateway_root_pda,
        &domain_separator,
        &raw_execute_data,
    )
    .expect("failed to create initialize_execute_data instruction");
    let BanksTransactionResultWithMetadata { metadata, result } = fixture
        .send_tx_with_metadata(&[
            ComputeBudgetInstruction::set_compute_unit_limit(1555555),
            ix,
        ])
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
    let quorum = 14;
    let nonce = 312;
    let domain_separator = [32; 32];
    let signers = make_signers_with_quorum(&[10, 4], nonce, quorum);
    let (uninitialized_gateway_config_pda, _) = GatewayConfig::pda();
    let (payload, _) = make_payload_and_commands(1);
    let (raw_execute_data, _) = prepare_execute_data(payload, &signers, &domain_separator);

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_approve_messages_execute_data(
        fixture.payer.pubkey(),
        uninitialized_gateway_config_pda,
        &domain_separator,
        &raw_execute_data,
    )
    .expect("failed to create initialize_execute_data instruction");

    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&[ix]).await;

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

    // Action
    let (payload, _) = make_payload_and_commands(1);
    // We init the execute data account (the helper method sends a tx to the
    // gateway program)
    let (_, raw_execute_data) = fixture
        .init_execute_data(&gateway_root_pda, payload, &signers, &domain_separator)
        .await;

    // We try to init the execute data account again with the same data
    let (ix, _) = gmp_gateway::instructions::initialize_approve_messages_execute_data(
        fixture.payer.pubkey(),
        gateway_root_pda,
        &domain_separator,
        &raw_execute_data,
    )
    .expect("failed to create initialize_execute_data instruction");
    let BanksTransactionResultWithMetadata { metadata, result } =
        fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(result.is_err(), "Transaction should have failed");
    assert!(metadata
        .unwrap()
        .log_messages
        .into_iter()
        .any(|x| x.contains("invalid account data for instruction")),);
}
