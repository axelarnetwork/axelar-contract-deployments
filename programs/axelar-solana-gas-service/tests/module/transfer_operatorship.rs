use axelar_solana_gas_service::instructions::transfer_operatorship;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_successfully_transfer_operatorship() {
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;

    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    let new_operator = Keypair::new();
    let ix = transfer_operatorship(&gas_utils.operator.pubkey(), &new_operator.pubkey()).unwrap();

    test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[&test_fixture.payer.insecure_clone(), &gas_utils.operator],
        )
        .await
        .unwrap();

    let config = test_fixture
        .gas_service_config_state(gas_utils.config_pda)
        .await;
    assert_eq!(config.operator, new_operator.pubkey());
}

#[tokio::test]
async fn test_fail_transfer_operatorship_invalid_operator() {
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;

    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    let new_operator = Keypair::new();
    let wrong_operator = Keypair::new();
    let ix = transfer_operatorship(&wrong_operator.pubkey(), &new_operator.pubkey()).unwrap();

    let result = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[&test_fixture.payer.insecure_clone(), &wrong_operator],
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_fail_transfer_operatorship_missing_signature() {
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;

    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    let new_operator = Keypair::new();
    let mut ix =
        transfer_operatorship(&gas_utils.operator.pubkey(), &new_operator.pubkey()).unwrap();
    ix.accounts[0].is_signer = false;

    let result = test_fixture
        .send_tx_with_custom_signers(&[ix], &[&test_fixture.payer.insecure_clone()])
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_successfully_transfer_and_use_new_operator() {
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;

    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    let new_operator = Keypair::new();
    let initial_new_operator_balance = 10_000_000_000;
    test_fixture
        .fund_account(&new_operator.pubkey(), initial_new_operator_balance)
        .await;

    let ix = transfer_operatorship(&gas_utils.operator.pubkey(), &new_operator.pubkey()).unwrap();
    test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[&test_fixture.payer.insecure_clone(), &gas_utils.operator],
        )
        .await
        .unwrap();

    let amount = 1_000_000_000;
    let initial_config_pda_balance = amount * 2;

    test_fixture
        .fund_account(&gas_utils.config_pda, initial_config_pda_balance)
        .await;

    let config_pda_balance_before = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let new_operator_balance_before = test_fixture
        .try_get_account_no_checks(&new_operator.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let collect_ix = axelar_solana_gas_service::instructions::collect_fees_instruction(
        &new_operator.pubkey(),
        &new_operator.pubkey(),
        amount,
    )
    .unwrap();

    test_fixture
        .send_tx_with_custom_signers(
            &[collect_ix],
            &[&test_fixture.payer.insecure_clone(), &new_operator],
        )
        .await
        .unwrap();

    let config_pda_balance_after = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let new_operator_balance_after = test_fixture
        .try_get_account_no_checks(&new_operator.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        config_pda_balance_after,
        config_pda_balance_before - amount,
        "Config PDA balance should decrease by the fee amount"
    );
    assert_eq!(
        new_operator_balance_after,
        new_operator_balance_before + amount,
        "New operator balance should increase by the fee amount"
    );

    // Verify old operator cannot collect fees
    let collect_ix = axelar_solana_gas_service::instructions::collect_fees_instruction(
        &gas_utils.operator.pubkey(),
        &gas_utils.operator.pubkey(),
        amount,
    )
    .unwrap();

    let result = test_fixture
        .send_tx_with_custom_signers(
            &[collect_ix],
            &[&test_fixture.payer.insecure_clone(), &gas_utils.operator],
        )
        .await;

    assert!(
        result.is_err(),
        "Old operator should not be able to collect fees"
    );
}
