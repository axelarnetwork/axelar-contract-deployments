use axelar_message_primitives::U256;
use axelar_solana_gateway::get_gateway_root_config_pda;
use axelar_solana_gateway::state::verifier_set_tracker::VerifierSetTracker;
use axelar_solana_gateway::state::GatewayConfig;
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

fn cmp_config(init: &SolanaAxelarIntegrationMetadata, created: &GatewayConfig) -> bool {
    let current_epoch: U256 = U256::ONE;
    let previous_signers_retention: U256 = init.previous_signers_retention.into();
    created.operator == init.operator.pubkey()
        && created.domain_separator == init.domain_separator
        && created.auth_weighted.current_epoch == current_epoch
        && created.auth_weighted.previous_signers_retention == previous_signers_retention
        && created.auth_weighted.minimum_rotation_delay == init.minimum_rotate_signers_delay_seconds
        // this just checks that the last rotation ts has been set to a non-zero value
        && created.auth_weighted.last_rotation_timestamp > 0
}

async fn assert_verifier_sets(metadata: &mut SolanaAxelarIntegrationMetadata) {
    let vs_data = metadata.init_gateway_config_verifier_set_data();
    for (idx, (verifier_set_hash, pda, bump)) in vs_data.into_iter().enumerate() {
        // assert_eq!(bump, bump2, "bumps don't match");
        let vst = metadata.get_account(&pda, &axelar_solana_gateway::ID).await;
        let vs_data = borsh::from_slice::<VerifierSetTracker>(&vst.data).unwrap();
        let epoch = U256::from_u64(idx as u64 + 1);
        assert_eq!(
            vs_data,
            VerifierSetTracker {
                bump,
                epoch,
                verifier_set_hash
            },
            "verifier set tracker not properly initialized"
        );
    }
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_single_initial_signer() {
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup_without_init_config()
        .await;
    let (gateway_config_pda, _bump) = get_gateway_root_config_pda();
    let initial_sets = metadata.init_gateway_config_verifier_set_data();
    let ix = axelar_solana_gateway::instructions::initialize_config(
        metadata.fixture.payer.pubkey(),
        metadata.domain_separator,
        initial_sets,
        metadata.minimum_rotate_signers_delay_seconds,
        metadata.operator.pubkey(),
        metadata.previous_signers_retention.into(),
        gateway_config_pda,
    )
    .unwrap();

    metadata.send_tx(&[ix]).await.unwrap();

    // Assert -- config derived correctly
    let root_pda_data = metadata
        .fixture
        .get_account(&gateway_config_pda, &axelar_solana_gateway::ID)
        .await;
    let root_pda_data = borsh::from_slice::<GatewayConfig>(&root_pda_data.data).unwrap();
    assert!(cmp_config(&metadata, &root_pda_data));

    // Assert -- block timestamp updated
    let clock = metadata.banks_client.get_sysvar::<Clock>().await.unwrap();
    let block_timestamp = clock.unix_timestamp as u64;
    assert_eq!(
        root_pda_data.last_rotation_timestamp, block_timestamp,
        "timestamp invalid"
    );

    // Assert -- epoch set to the one we expect
    let current_epoch = U256::from(1_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);

    // Assert -- verifier set PDAs are initialized
    assert_verifier_sets(&mut metadata).await;
}

#[tokio::test]
async fn test_reverts_on_invalid_gateway_pda_pubkey() {
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup_without_init_config()
        .await;
    let initial_sets = metadata.init_gateway_config_verifier_set_data();
    let ix = axelar_solana_gateway::instructions::initialize_config(
        metadata.fixture.payer.pubkey(),
        metadata.domain_separator,
        initial_sets,
        metadata.minimum_rotate_signers_delay_seconds,
        metadata.operator.pubkey(),
        metadata.previous_signers_retention.into(),
        Pubkey::new_unique(),
    )
    .unwrap();

    let res = metadata.send_tx(&[ix]).await.expect_err("tx should fail");

    // Assert
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.to_lowercase().contains("invalid gateway root pda")),
        "Expected error message not found!"
    );
}

#[tokio::test]
async fn test_reverts_on_already_initialized_gateway_pda() {
    let mut metadata = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![42])
        .build()
        .setup()
        .await;
    let (gateway_config_pda, _bump) = get_gateway_root_config_pda();
    let initial_sets = metadata.init_gateway_config_verifier_set_data();
    let ix = axelar_solana_gateway::instructions::initialize_config(
        metadata.fixture.payer.pubkey(),
        metadata.domain_separator,
        initial_sets,
        metadata.minimum_rotate_signers_delay_seconds,
        metadata.operator.pubkey(),
        metadata.previous_signers_retention.into(),
        gateway_config_pda,
    )
    .unwrap();
    let res = metadata.send_tx(&[ix]).await.expect_err("tx should fail");

    // Assert
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.contains("invalid account data for instruction")),
        "Expected error message not found!"
    );
}
