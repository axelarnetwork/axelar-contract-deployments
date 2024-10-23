use axelar_message_primitives::U256;
use axelar_rkyv_encoding::hasher::merkle_trait::Merkle;
use axelar_rkyv_encoding::hasher::merkle_tree::NativeHasher;
use axelar_rkyv_encoding::test_fixtures::random_valid_verifier_set;
use axelar_solana_gateway::instructions::InitializeConfig;
use axelar_solana_gateway::state::verifier_set_tracker::{VerifierSetHash, VerifierSetTracker};
use axelar_solana_gateway::state::GatewayConfig;
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::runner::TestRunner;

const DOMAIN_SEPARATOR: [u8; 32] = [42; 32];

fn cmp_config(init: &InitializeConfig<VerifierSetHash>, created: &GatewayConfig) -> bool {
    let current_epoch: U256 = init.initial_signer_sets.len().into();
    created.operator == init.operator
        && created.domain_separator == init.domain_separator
        && created.auth_weighted.current_epoch == current_epoch
        && created.auth_weighted.previous_signers_retention == init.previous_signers_retention
        && created.auth_weighted.minimum_rotation_delay == init.minimum_rotation_delay
        // this just checks that the last rotation ts has been set to a non-zero value
        && created.auth_weighted.last_rotation_timestamp > 0
}

fn make_initialize_config<T>(init: T) -> InitializeConfig<T> {
    InitializeConfig {
        domain_separator: DOMAIN_SEPARATOR,
        initial_signer_sets: vec![init],
        minimum_rotation_delay: 0,
        operator: Pubkey::new_unique(),
        previous_signers_retention: 0u128.into(),
    }
}

async fn assert_verifier_sets(
    init_config: InitializeConfig<VerifierSetHash>,
    runner: &mut TestRunner,
) {
    let (cnf, pdas) = init_config.with_verifier_set_bump();
    for (idx, ((pda, bump), (vs, bump2))) in
        pdas.into_iter().zip(cnf.initial_signer_sets).enumerate()
    {
        assert_eq!(bump, bump2, "bumps don't match");
        let vst = runner
            .get_account::<VerifierSetTracker>(&pda, &axelar_solana_gateway::ID)
            .await;
        let epoch = U256::from_u64(idx as u64 + 1);
        assert_eq!(
            vst,
            VerifierSetTracker {
                bump,
                epoch,
                verifier_set_hash: vs
            },
            "verifier set tracker not properly initialized"
        );
    }
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_single_initial_signer() {
    let mut runner = TestRunner::new().await;
    let (gateway_config_pda, _bump) = GatewayConfig::pda();

    let initial_signer_set = random_valid_verifier_set();
    let initial_signer_set_root =
        Merkle::<NativeHasher>::calculate_merkle_root(&initial_signer_set)
            .expect("expected a non-empty signer set");

    let initial_config: InitializeConfig<VerifierSetHash> =
        make_initialize_config(initial_signer_set_root);

    let ix = axelar_solana_gateway::instructions::initialize_config(
        runner.payer.pubkey(),
        initial_config.clone(),
        gateway_config_pda,
    )
    .unwrap();

    runner.send_tx(&[ix]).await;

    // Assert -- config derived correctly
    let root_pda_data = runner
        .get_account::<axelar_solana_gateway::state::GatewayConfig>(
            &gateway_config_pda,
            &axelar_solana_gateway::ID,
        )
        .await;
    assert!(cmp_config(&initial_config, &root_pda_data));

    // Assert -- block timestamp updated
    let clock = runner.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp as u64;
    assert_eq!(
        root_pda_data.last_rotation_timestamp, block_timestamp,
        "timestamp invalid"
    );

    // Assert -- epoch set to the one we expect
    let current_epoch = U256::from(1_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(initial_config, &mut runner).await;
}

#[tokio::test]
async fn test_reverts_on_invalid_gateway_pda_pubkey() {
    // Setup
    let mut runner = TestRunner::new().await;
    let initial_signer_set = random_valid_verifier_set();

    let (_gateway_config_pda, _bump) = GatewayConfig::pda();

    let initial_signer_set_root =
        Merkle::<NativeHasher>::calculate_merkle_root(&initial_signer_set)
            .expect("expected a non-empty signer set");

    let initial_config: InitializeConfig<VerifierSetHash> =
        make_initialize_config(initial_signer_set_root);

    let ix = axelar_solana_gateway::instructions::initialize_config(
        runner.payer.pubkey(),
        initial_config.clone(),
        Pubkey::new_unique(),
    )
    .unwrap();

    let res = runner.send_tx_with_metadata(&[ix]).await;

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
    let mut runner = TestRunner::new().await;
    let initial_signer_set = random_valid_verifier_set();

    let initial_signer_set_root =
        Merkle::<NativeHasher>::calculate_merkle_root(&initial_signer_set)
            .expect("expected a non-empty signer set");

    let initial_config = make_initialize_config(initial_signer_set_root);

    let gateway_config_pda = runner
        .initialize_gateway_config_account(initial_config.clone())
        .await;

    // Action
    let ix = axelar_solana_gateway::instructions::initialize_config(
        runner.payer.pubkey(),
        initial_config,
        gateway_config_pda,
    )
    .unwrap();
    let res = runner.send_tx_with_metadata(&[ix]).await;

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
