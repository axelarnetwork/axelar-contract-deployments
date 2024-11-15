use governance::instructions::builder::IxBuilder;
use governance::state::GovernanceConfig;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use test_fixtures::test_setup::TestFixture;

use crate::fixtures::MINIMUM_PROPOSAL_DELAY;
use crate::helpers::program_test;

#[tokio::test]
async fn test_successfully_initialize_config() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (config_pda, bump) = GovernanceConfig::pda();

    let config = governance::state::GovernanceConfig::new(
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
        .get_rkyv_account::<governance::state::GovernanceConfig>(&config_pda, &governance::ID)
        .await;
    assert_eq!(&config, &root_pda_data);
}
