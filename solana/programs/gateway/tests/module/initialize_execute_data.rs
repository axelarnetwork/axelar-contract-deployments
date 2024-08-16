use axelar_rkyv_encoding::types::Payload;
use gmp_gateway::instructions::InitializeConfig;
use gmp_gateway::state::{GatewayConfig, GatewayExecuteData};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata, ProgramTestBanksClientExt};
use solana_sdk::account::Account;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::system_program;
use test_fixtures::axelar_message::new_signer_set;
use test_fixtures::execute_data::prepare_execute_data;
use test_fixtures::test_setup::TestFixture;

use crate::{
    create_signer_with_weight, make_messages, make_payload_and_commands, make_signers,
    program_test, setup_initialised_gateway, InitialisedGatewayMetadata,
};

#[tokio::test]
async fn test_successfylly_initialize_execute_data() {
    // Setup
    let InitialisedGatewayMetadata {
        nonce,
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[10, 4], None).await;
    let payload = Payload::new_messages(make_messages(1));
    let (raw_execute_data, _) =
        prepare_execute_data(payload, &signers, quorum, nonce, &fixture.domain_separator);
    let gateway_execute_data = GatewayExecuteData::new(
        &raw_execute_data,
        &gateway_root_pda,
        &fixture.domain_separator,
    )
    .expect("valid GatewayExecuteData");

    let (execute_data_pda, _) = gateway_execute_data.pda(&gateway_root_pda);

    // Action
    fixture
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(1_399_850_u32),
            gmp_gateway::instructions::initialize_execute_data(
                fixture.payer.pubkey(),
                gateway_root_pda,
                &fixture.domain_separator,
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
    let deserialized_gateway_execute_data = GatewayExecuteData::new(
        account.data.as_slice(),
        &gateway_root_pda,
        &fixture.domain_separator,
    )
    .expect("GatewayExecuteData can be deserialized");
    assert_eq!(deserialized_gateway_execute_data, gateway_execute_data);
}

#[tokio::test]
async fn test_succesfully_initialize_rotate_signers() {
    // Setup
    let InitialisedGatewayMetadata {
        nonce,
        mut fixture,
        quorum,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[10, 4], None).await;
    let new_signers = make_signers(&[33, 150]);

    let payload = Payload::VerifierSet(new_signer_set(&new_signers, 0, quorum));

    let (raw_execute_data, _) =
        prepare_execute_data(payload, &signers, quorum, nonce, &fixture.domain_separator);
    let gateway_execute_data = GatewayExecuteData::new(
        &raw_execute_data,
        &gateway_root_pda,
        &fixture.domain_separator,
    )
    .expect("valid GatewayExecuteData");
    let (execute_data_pda, _) = gateway_execute_data.pda(&gateway_root_pda);

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
        fixture.payer.pubkey(),
        gateway_root_pda,
        &fixture.domain_separator,
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
    let deserialized_gateway_execute_data = GatewayExecuteData::new(
        account.data.as_slice(),
        &gateway_root_pda,
        &fixture.domain_separator,
    )
    .expect("GatewayExecuteData can be deserialized");
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
    let signers = make_signers(&[10, 4]);
    let threshold = 14;
    let nonce = 123;
    fixture
        .initialize_gateway_config_account(InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[(&signers, nonce)]),
            ..fixture.base_initialize_config()
        })
        .await;
    let (payload, _) = make_payload_and_commands(1);
    let (raw_execute_data, _) = prepare_execute_data(
        payload,
        &signers,
        threshold,
        nonce,
        &fixture.domain_separator,
    );

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
        fixture.payer.pubkey(),
        fake_gateway_root_pda,
        &fixture.domain_separator,
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
    let signers = make_signers(&[10, 4]);
    let threshold = 14;
    let nonce = 123321;
    fixture
        .initialize_gateway_config_account(InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[(&signers, nonce)]),
            ..fixture.base_initialize_config()
        })
        .await;
    let (payload, _) = make_payload_and_commands(1);
    let (raw_execute_data, _) = prepare_execute_data(
        payload,
        &signers,
        threshold,
        nonce,
        &fixture.domain_separator,
    );

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
        fixture.payer.pubkey(),
        fake_gateway_root_pda,
        // gateway_root_pda,
        &fixture.domain_separator,
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
    let signers = make_signers(&[10, 14]);
    let threshold = 14;
    let nonce = 312;
    let (uninitialized_gateway_config_pda, _) = GatewayConfig::pda();
    let (payload, _) = make_payload_and_commands(1);
    let (raw_execute_data, _) = prepare_execute_data(
        payload,
        &signers,
        threshold,
        nonce,
        &fixture.domain_separator,
    );

    // Action
    let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
        fixture.payer.pubkey(),
        uninitialized_gateway_config_pda,
        &fixture.domain_separator,
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
    let InitialisedGatewayMetadata {
        nonce,
        mut fixture,
        quorum: threshold,
        signers,
        gateway_root_pda,
        ..
    } = setup_initialised_gateway(&[10, 4], None).await;

    // Action
    let domain_separator = fixture.domain_separator;
    let (payload, _) = make_payload_and_commands(1);
    // We init the execute data account (the helper method sends a tx to the
    // gateway program)
    let (_, raw_execute_data) = fixture
        .init_execute_data(
            &gateway_root_pda,
            payload,
            &signers,
            threshold,
            nonce,
            &domain_separator,
        )
        .await;

    // We try to init the execute data account again with the same data
    let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
        fixture.payer.pubkey(),
        gateway_root_pda,
        &fixture.domain_separator,
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

/// processing any more than 19 signers results in `memory allocation failed,
/// out of memory` Which means that we exceeded the 32kb heap memory limit
/// [docs](https://solana.com/docs/programs/faq#heap-size)
///
/// Technically we could try using a custom allocator to clean up the heap
/// because we still have a lot of compute budget to work with:
/// `consumed 690929 of 1399850 compute units` - on 33 signer amount
///
/// 1399850 - this is the maximum amount of compute units that we can use, if we
/// try setting a larger value, it just gets rounded to this one.
#[tokio::test]
async fn test_size_limits_for_different_signers() {
    // Setup
    let nonce = 4444;
    for amount_of_signers in [2, 4, 8, 16, 17, 18, 19] {
        dbg!(amount_of_signers);
        let signers = (0..amount_of_signers)
            .map(|x| create_signer_with_weight(x + 1))
            .collect::<Vec<_>>();
        let threshold = (0..amount_of_signers).sum::<u128>() + amount_of_signers;
        let mut fixture = TestFixture::new(program_test()).await;
        let gateway_root_pda = fixture
            .initialize_gateway_config_account(InitializeConfig {
                initial_signer_sets: fixture.create_verifier_sets(&[(&signers, nonce)]),
                ..fixture.base_initialize_config()
            })
            .await;

        let (payload, _) = make_payload_and_commands(1);
        let (raw_execute_data, _) = prepare_execute_data(
            payload,
            &signers,
            threshold,
            nonce,
            &fixture.domain_separator,
        );
        let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            gateway_root_pda,
            &fixture.domain_separator,
            &raw_execute_data,
        )
        .expect("failed to create initialize_execute_data instruction");
        fixture.recent_blockhash = fixture
            .banks_client
            .get_new_latest_blockhash(&fixture.recent_blockhash)
            .await
            .unwrap();
        fixture
            .send_tx(&[
                // add compute budget increase
                ComputeBudgetInstruction::set_compute_unit_limit(1399850_u32),
                ix,
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
    let nonce = 123321;
    for amount_of_messages in [1, 2, 4, 8, 15] {
        dbg!(amount_of_messages);
        let signers = vec![create_signer_with_weight(4_u128)];
        let threshold = 4;
        let mut fixture = TestFixture::new(program_test()).await;
        let gateway_root_pda = fixture
            .initialize_gateway_config_account(InitializeConfig {
                initial_signer_sets: fixture.create_verifier_sets(&[(&signers, nonce)]),
                ..fixture.base_initialize_config()
            })
            .await;

        let messages = make_messages(amount_of_messages);
        let payload = Payload::new_messages(messages);

        let (raw_execute_data, _) = prepare_execute_data(
            payload,
            &signers,
            threshold,
            nonce,
            &fixture.domain_separator,
        );
        fixture.recent_blockhash = fixture
            .banks_client
            .get_new_latest_blockhash(&fixture.recent_blockhash)
            .await
            .unwrap();
        let (ix, _) = gmp_gateway::instructions::initialize_execute_data(
            fixture.payer.pubkey(),
            gateway_root_pda,
            &fixture.domain_separator,
            &raw_execute_data,
        )
        .expect("failed to create initialize_execute_data instruction");
        fixture
            .send_tx(&[
                // add compute budget increase
                ComputeBudgetInstruction::set_compute_unit_limit(1399850_u32),
                ix,
            ])
            .await;
    }
}
