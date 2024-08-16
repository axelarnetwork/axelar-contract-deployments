use axelar_message_primitives::command::U256;
use gmp_gateway::hasher_impl;
use gmp_gateway::instructions::InitializeConfig;
use gmp_gateway::state::GatewayConfig;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::axelar_message::new_signer_set;
use test_fixtures::test_setup::TestFixture;

use crate::{create_signer_with_weight, make_signers, program_test};

const NONCE: u64 = 44;
fn cmp_config(init: &InitializeConfig, created: &GatewayConfig) -> bool {
    let all_hashes_present_in_order =
        init.initial_signer_sets
            .iter()
            .enumerate()
            .all(|(idx, verifier_set)| {
                let left = verifier_set
                    .parse()
                    .expect("provided invalid verifier set")
                    .hash(hasher_impl());
                let Some(epoch) = created.signer_sets().get_by_left(&left) else {
                    return false;
                };
                let idx = U256::from(idx + 1);
                &idx == epoch
            });
    let current_epoch = init.initial_signer_sets.len().into();
    let last_rotation_timestamp = 0;
    created.operator == init.operator
        && created.domain_separator == init.domain_separator
        && created.auth_weighted.current_epoch == current_epoch
        && created.auth_weighted.previous_signers_retention == init.previous_signers_retention
        && created.auth_weighted.minimum_rotation_delay == init.minimum_rotation_delay
        && created.auth_weighted.last_rotation_timestamp == last_rotation_timestamp
        && all_hashes_present_in_order
}

#[tokio::test]
async fn test_successfylly_initialize_config() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = make_signers(&[10, 4]);
    let new_signer_set = new_signer_set(&initial_signers, NONCE, 14);
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[(&initial_signers, NONCE)]),
        ..fixture.base_initialize_config()
    };
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        init_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_config_pda, &gmp_gateway::ID)
        .await;
    assert!(cmp_config(&init_config, &root_pda_data));

    let current_epoch = U256::from(1_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);
    assert_eq!(
        root_pda_data
            .auth_weighted
            .signer_set_hash_for_epoch(&current_epoch)
            .unwrap(),
        &new_signer_set.hash(hasher_impl()),
    );
}

#[tokio::test]
async fn test_successfylly_initialize_config_without_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = vec![];
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[(&initial_signers, NONCE)]),
        ..fixture.base_initialize_config()
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
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_25_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = (0..25)
        .map(|_| create_signer_with_weight(10_u128))
        .collect::<Vec<_>>();
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets(&[(&initial_signers, NONCE)]),
        ..fixture.base_initialize_config()
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
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_25_signers_custom_small_threshold() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let threshold = 1_u128;
    let initial_signers = (0..25)
        .map(|_| create_signer_with_weight(10_u128))
        .collect::<Vec<_>>();
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets_with_thershold(&[(
            &initial_signers,
            NONCE,
            threshold.into(),
        )]),
        ..fixture.base_initialize_config()
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
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_25_signers_custom_large_threshold() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let threshold = u128::MAX;
    let initial_signers = (0..25)
        .map(|_| create_signer_with_weight(10_u128))
        .collect::<Vec<_>>();
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let init_config = InitializeConfig {
        initial_signer_sets: fixture.create_verifier_sets_with_thershold(&[(
            &initial_signers,
            NONCE,
            threshold.into(),
        )]),
        ..fixture.base_initialize_config()
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
}

#[tokio::test]
async fn test_reverts_on_invalid_gateway_pda_pubkey() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let (_gateway_config_pda, _bump) = GatewayConfig::pda();

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[(&initial_signers, NONCE)]),
            ..fixture.base_initialize_config()
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
    let initial_signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[(&initial_signers, NONCE)]),
            ..fixture.base_initialize_config()
        })
        .await;

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        InitializeConfig {
            initial_signer_sets: fixture.create_verifier_sets(&[(&initial_signers, NONCE)]),
            ..fixture.base_initialize_config()
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
