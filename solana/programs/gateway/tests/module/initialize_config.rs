use axelar_message_primitives::command::U256;
use gmp_gateway::hasher_impl;
use gmp_gateway::instructions::{InitializeConfig, VerifierSetWraper};
use gmp_gateway::state::verifier_set_tracker::VerifierSetTracker;
use gmp_gateway::state::GatewayConfig;
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::test_setup::{make_signers, make_signers_with_quorum, TestFixture};

use crate::program_test;

const NONCE: u64 = 44;
const DOMAIN_SEPARATOR: [u8; 32] = [42; 32];

fn cmp_config(init: &InitializeConfig<VerifierSetWraper>, created: &GatewayConfig) -> bool {
    let current_epoch = init.initial_signer_sets.len().into();
    created.operator == init.operator
        && created.domain_separator == init.domain_separator
        && created.auth_weighted.current_epoch == current_epoch
        && created.auth_weighted.previous_signers_retention == init.previous_signers_retention
        && created.auth_weighted.minimum_rotation_delay == init.minimum_rotation_delay
        // this just checks that the last rotation ts has been set to a non-zero value
        && created.auth_weighted.last_rotation_timestamp > 0
}

async fn assert_verifier_sets(
    init_config: InitializeConfig<VerifierSetWraper>,
    fixture: &mut TestFixture,
) {
    let (cnf, pdas) = init_config.with_verifier_set_bump();
    for (idx, ((pda, bump), (vs, bump2))) in
        pdas.into_iter().zip(cnf.initial_signer_sets).enumerate()
    {
        assert_eq!(bump, bump2, "bumps don't match");
        let vst = fixture
            .get_account::<VerifierSetTracker>(&pda, &gmp_gateway::ID)
            .await;
        let epoch = U256::from_u64(idx as u64 + 1);
        assert_eq!(
            vst,
            VerifierSetTracker {
                bump,
                epoch,
                verifier_set_hash: vs.parse().unwrap().hash(hasher_impl())
            },
            "verifier set tracker not properly initialized"
        );
    }
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_single_initial_signer() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = make_signers(&[10, 4], NONCE);
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
        ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert -- config derived correctly
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_config_pda, &gmp_gateway::ID)
        .await;
    assert!(cmp_config(&init_config, &root_pda_data));

    // Assert -- blokc timestamp updated
    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp as u64;
    assert_eq!(
        root_pda_data.last_rotation_timestamp, block_timestamp,
        "timestamp invalid"
    );

    // Assert -- epoch set to the one we expect
    let current_epoch = U256::from(1_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(init_config, &mut fixture).await;
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_multiple_initial_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[
            &make_signers(&[10, 4], 8),
            &make_signers(&[22, 114], 16),
            &make_signers(&[10, 5], 18),
        ]),
        ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert -- config derived correctly
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_config_pda, &gmp_gateway::ID)
        .await;
    assert!(cmp_config(&init_config, &root_pda_data));

    // Assert -- blokc timestamp updated
    let clock = fixture.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp as u64;
    assert_eq!(
        root_pda_data.last_rotation_timestamp, block_timestamp,
        "timestamp invalid"
    );

    // Assert -- epoch set to the one we expect
    let current_epoch = U256::from(3_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(init_config, &mut fixture).await;
}

#[tokio::test]
async fn test_successfylly_initialize_config_without_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: vec![],
        ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(gateway_config_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
    assert!(cmp_config(&init_config, &deserialized_gateway_config));

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(init_config, &mut fixture).await;
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_25_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_weights = (0..25).map(|_| 10).collect::<Vec<_>>();
    let initial_signers = make_signers(&initial_weights, NONCE);
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
        ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(gateway_config_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
    assert!(cmp_config(&init_config, &deserialized_gateway_config));

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(init_config, &mut fixture).await;
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_25_signers_custom_small_threshold() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let quorum = 1_u128;
    let initial_weights = (0..25).map(|_| 10).collect::<Vec<_>>();
    let initial_signers = make_signers_with_quorum(&initial_weights, NONCE, quorum);
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
        ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(gateway_config_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
    assert!(cmp_config(&init_config, &deserialized_gateway_config));

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(init_config, &mut fixture).await;
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_25_signers_custom_large_threshold() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let quorum = u128::MAX;
    let initial_weights = (0..25).map(|_| 10).collect::<Vec<_>>();
    let initial_signers = make_signers_with_quorum(&initial_weights, NONCE, quorum);
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
        ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert
    let account = fixture
        .banks_client
        .get_account(gateway_config_pda)
        .await
        .unwrap()
        .expect("metadata");
    assert_eq!(account.owner, gmp_gateway::id());
    let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
    assert!(cmp_config(&init_config, &deserialized_gateway_config));

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(init_config, &mut fixture).await;
}

#[tokio::test]
async fn test_reverts_on_invalid_gateway_pda_pubkey() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = make_signers(&[10, 4], NONCE);

    let (_gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
            ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
        },
        Pubkey::new_unique(),
    )
    .unwrap();
    let res = fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(res.result.is_err(), "Transaction should fail");
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.contains("invalid gateway root pda")),
        "Expected error message not found!"
    );
}

#[tokio::test]
async fn test_reverts_on_already_initialized_gateway_pda() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = make_signers(&[10, 4], NONCE);
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
            ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
        })
        .await;

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[&initial_signers]),
            ..fixture.base_initialize_config(DOMAIN_SEPARATOR)
        },
        gateway_config_pda,
    )
    .unwrap();
    let res = fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(res.result.is_err(), "Transaction should fail");
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.contains("invalid account data for instruction")),
        "Expected error message not found!"
    );
}
