use axelar_solana_gas_service::state::Config;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{keccak::hashv, signer::Signer};

#[tokio::test]
async fn test_successfylly_initialize_config() {
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
            authority: gas_utils.config_authority.pubkey(),
            salt: gas_utils.salt,
            bump: config.bump
        }
    );
}

#[tokio::test]
async fn test_different_salts_give_new_configs() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;

    // Action
    let salt_seeds = b"abc";
    for salt_seed in salt_seeds {
        let salt = hashv(&[&[*salt_seed]]).0;
        let (config_pda, bump) = axelar_solana_gas_service::get_config_pda(
            &axelar_solana_gas_service::ID,
            &salt,
            &gas_utils.config_authority.pubkey(),
        );
        let _res = test_fixture
            .init_gas_config_with_params(gas_utils.config_authority.pubkey(), config_pda, salt)
            .await
            .unwrap();
        // Assert
        let config = test_fixture.gas_service_config_state(config_pda).await;
        assert_eq!(
            config,
            Config {
                authority: gas_utils.config_authority.pubkey(),
                salt,
                bump
            }
        );
    }

    // assert -- subsequent initializations will revert the tx
    for salt_seed in salt_seeds {
        let salt = hashv(&[&[*salt_seed]]).0;
        let (config_pda, _bump) = axelar_solana_gas_service::get_config_pda(
            &axelar_solana_gas_service::ID,
            &salt,
            &gas_utils.config_authority.pubkey(),
        );
        let res = test_fixture
            .init_gas_config_with_params(gas_utils.config_authority.pubkey(), config_pda, salt)
            .await;
        assert!(res.is_err());
    }
}
