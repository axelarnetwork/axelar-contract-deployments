use axelar_solana_gas_service::state::Config;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::signer::Signer;

#[tokio::test]
async fn test_successfully_initialize_config() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;

    // Action
    let _res = test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Assert
    let config = test_fixture
        .gas_service_config_state(gas_utils.config_pda)
        .await;
    assert_eq!(
        config,
        Config {
            operator: gas_utils.operator.pubkey(),
            bump: config.bump
        }
    );
}
