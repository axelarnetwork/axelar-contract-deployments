use axelar_solana_governance::instructions::builder::IxBuilder;
use axelar_solana_governance::state::GovernanceConfig;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::test_setup::TestFixture;

use crate::fixtures::MINIMUM_PROPOSAL_DELAY;
use crate::helpers::{assert_msg_present_in_logs, program_test};

#[tokio::test]
async fn test_successfully_initialize_config() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (config_pda, bump) = GovernanceConfig::pda();

    let config = axelar_solana_governance::state::GovernanceConfig::new(
        bump,
        [0_u8; 32],
        [0_u8; 32],
        MINIMUM_PROPOSAL_DELAY,
        Pubkey::new_unique().to_bytes(),
    );

    let ix = IxBuilder::new()
        .initialize_config(&fixture.payer.pubkey(), &config_pda, config.clone())
        .build();

    let res = fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(res.result.is_ok());
    let root_pda_data = fixture
        .get_rkyv_account::<axelar_solana_governance::state::GovernanceConfig>(
            &config_pda,
            &axelar_solana_governance::ID,
        )
        .await;
    assert_eq!(&config, &root_pda_data);
}

#[tokio::test]
async fn test_program_checks_config_pda_successfully_derived() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (_, bump) = GovernanceConfig::pda();

    let config = axelar_solana_governance::state::GovernanceConfig::new(
        bump,
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

    let res = fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Derived PDA does not match provided PDA");
}
