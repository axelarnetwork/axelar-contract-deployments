use axelar_message_primitives::command::U256;
use gmp_gateway::state::GatewayConfig;
use solana_program_test::{tokio, ProgramTestBanksClientExt};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::axelar_message::new_signer_set;
use test_fixtures::test_setup::TestFixture;

use crate::{create_signer_with_weight, make_signers, program_test};

#[tokio::test]
async fn test_successfylly_initialize_config() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = make_signers(&[10, 4]);
    let new_signer_set = new_signer_set(&initial_signers, 0, 14);
    let (gateway_config_pda, bump) = GatewayConfig::pda();
    let auth_weighted = fixture.init_auth_weighted_module(&initial_signers);
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
        gateway_config_pda,
    )
    .unwrap();
    fixture.send_tx(&[ix]).await;

    // Assert
    let root_pda_data = fixture
        .get_account::<gmp_gateway::state::GatewayConfig>(&gateway_config_pda, &gmp_gateway::ID)
        .await;
    assert_eq!(root_pda_data, gateway_config);

    let current_epoch = U256::from(1_u8);
    assert_eq!(root_pda_data.auth_weighted.current_epoch(), current_epoch);
    assert_eq!(
        root_pda_data
            .auth_weighted
            .signer_set_hash_for_epoch(&current_epoch)
            .unwrap(),
        &new_signer_set.hash(),
    );
}

#[tokio::test]
async fn test_successfylly_initialize_config_without_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = vec![];
    let (gateway_config_pda, bump) = GatewayConfig::pda();
    let auth_weighted = fixture.init_auth_weighted_module(&initial_signers);
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
    assert_eq!(deserialized_gateway_config, gateway_config);
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_50_signers() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = (0..50)
        .map(|_| create_signer_with_weight(10_u128))
        .collect::<Vec<_>>();
    let (gateway_config_pda, bump) = GatewayConfig::pda();
    let auth_weighted = fixture.init_auth_weighted_module(&initial_signers);
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
    assert_eq!(deserialized_gateway_config, gateway_config);
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_50_signers_custom_small_threshold() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let threshold = 1_u128;
    let initial_signers = (0..50)
        .map(|_| create_signer_with_weight(10_u128))
        .collect::<Vec<_>>();
    let (gateway_config_pda, bump) = GatewayConfig::pda();
    let auth_weighted =
        fixture.init_auth_weighted_module_custom_threshold(&initial_signers, threshold.into());
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
    assert_eq!(deserialized_gateway_config, gateway_config);
}

#[tokio::test]
async fn test_successfylly_initialize_config_with_50_signers_custom_large_threshold() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let threshold = u128::MAX;
    let initial_signers = (0..50)
        .map(|_| create_signer_with_weight(10_u128))
        .collect::<Vec<_>>();
    let (gateway_config_pda, bump) = GatewayConfig::pda();
    let auth_weighted =
        fixture.init_auth_weighted_module_custom_threshold(&initial_signers, threshold.into());
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
    assert_eq!(deserialized_gateway_config, gateway_config);
}

#[tokio::test]
async fn test_reverts_on_invalid_gateway_bump() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let (gateway_config_pda, bump) = GatewayConfig::pda();
    let invalid_bump = bump.overflowing_add(1).0;
    let auth_weighted = fixture.init_auth_weighted_module(&initial_signers);
    let gateway_config = GatewayConfig::new(
        invalid_bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
            .any(|x| x.contains("invalid bump for the root pda: InvalidSeeds")),
        "Expected error message not found!"
    );
}

#[tokio::test]
async fn test_reverts_on_invalid_gateway_pda_pubkey() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let initial_signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let (_gateway_config_pda, bump) = GatewayConfig::pda();
    let auth_weighted = fixture.init_auth_weighted_module(&initial_signers);
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
    let auth_weighted = fixture.init_auth_weighted_module(&initial_signers);
    let gateway_config_pda = fixture
        .initialize_gateway_config_account(auth_weighted.clone(), Pubkey::new_unique())
        .await;
    let (_, bump) = GatewayConfig::pda();
    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        Pubkey::new_unique(),
        fixture.domain_separator,
    );
    fixture.recent_blockhash = fixture
        .banks_client
        .get_new_latest_blockhash(&fixture.recent_blockhash)
        .await
        .unwrap();

    // Action
    let ix = gmp_gateway::instructions::initialize_config(
        fixture.payer.pubkey(),
        gateway_config.clone(),
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
