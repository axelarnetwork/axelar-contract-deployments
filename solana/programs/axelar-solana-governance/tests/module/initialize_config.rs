use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_governance::instructions::builder::IxBuilder;
use axelar_solana_governance::state::GovernanceConfig;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;

use crate::fixtures::MINIMUM_PROPOSAL_DELAY;
use crate::helpers::{assert_msg_present_in_logs, program_test};

#[tokio::test]
async fn test_successfully_initialize_config() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (config_pda, _) = GovernanceConfig::pda();

    let config = axelar_solana_governance::state::GovernanceConfig::new(
        [0_u8; 32],
        [0_u8; 32],
        MINIMUM_PROPOSAL_DELAY,
        Pubkey::new_unique().to_bytes(),
    );

    let ix = IxBuilder::new()
        .initialize_config(&fixture.payer.pubkey(), &config_pda, config.clone())
        .build();

    let res = fixture.send_tx(&[ix]).await;

    // Assert
    assert!(res.is_ok());
    let root_pda_data = fixture
        .get_account_with_borsh::<axelar_solana_governance::state::GovernanceConfig>(&config_pda)
        .await
        .unwrap();
    assert_eq!(&config.address_hash, &root_pda_data.address_hash);
    assert_eq!(&config.chain_hash, &root_pda_data.chain_hash);
    assert_eq!(
        &config.minimum_proposal_eta_delay,
        &root_pda_data.minimum_proposal_eta_delay
    );
    assert_eq!(&config.operator, &root_pda_data.operator);
}

#[tokio::test]
async fn test_program_checks_config_pda_successfully_derived() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;

    let config = axelar_solana_governance::state::GovernanceConfig::new(
        [0_u8; 32],
        [0_u8; 32],
        MINIMUM_PROPOSAL_DELAY,
        Pubkey::new_unique().to_bytes(),
    );

    let ix = IxBuilder::new()
        .initialize_config(
            &fixture.payer.pubkey(),
            &Pubkey::new_unique(),
            config.clone(),
        )
        .build(); // Wrong PDA

    let res = fixture.send_tx(&[ix]).await;

    // Assert
    assert!(res.is_err());
    assert_msg_present_in_logs(
        res.err().unwrap(),
        "Derived PDA does not match provided PDA",
    );
}

#[tokio::test]
async fn test_program_overrides_config_bump() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;

    let (config_pda, _) = GovernanceConfig::pda();

    let config = axelar_solana_governance::state::GovernanceConfig::new(
        [0_u8; 32],
        [0_u8; 32],
        MINIMUM_PROPOSAL_DELAY,
        Pubkey::new_unique().to_bytes(),
    );

    let ix = IxBuilder::new()
        .initialize_config(&fixture.payer.pubkey(), &config_pda, config.clone())
        .build(); // Wrong PDA

    let res = fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());

    let config = fixture
        .get_account_with_borsh::<axelar_solana_governance::state::GovernanceConfig>(&config_pda)
        .await
        .unwrap();

    // Assert
    assert!(config.bump != 0);
}
