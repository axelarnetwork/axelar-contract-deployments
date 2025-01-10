use axelar_solana_gateway_test_fixtures::base::TestFixture;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_receive_funds() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Record balances before the transaction
    test_fixture
        .fund_account(&gas_utils.config_pda, 1_000_000_000)
        .await;
    let receiver = Keypair::new();
    let receiver_balance_before = 0;
    let config_pda_balance_before = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Action
    let sol_amount = 1_000_000;
    let ix = axelar_solana_gas_service::instructions::collect_native_fees_instruction(
        &axelar_solana_gas_service::ID,
        &gas_utils.config_authority.pubkey(),
        &gas_utils.config_pda,
        &receiver.pubkey(),
        sol_amount,
    )
    .unwrap();

    test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
                // authority for config pda deduction
                &gas_utils.config_authority,
            ],
        )
        .await
        .unwrap();

    // assert that SOL gets transferred
    let receiver_balance_after = test_fixture
        .try_get_account_no_checks(&receiver.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let config_pda_balance_after = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    assert_eq!(
        config_pda_balance_after,
        config_pda_balance_before - sol_amount
    );
    assert_eq!(receiver_balance_after, receiver_balance_before + sol_amount);
}

#[tokio::test]
async fn test_refund_native_fails_if_not_signed_by_authority() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();
    test_fixture
        .fund_account(&gas_utils.config_pda, 1_000_000_000)
        .await;

    // Action
    let receiver = Keypair::new();
    let sol_amount = 1_000_000;
    let mut ix = axelar_solana_gas_service::instructions::collect_native_fees_instruction(
        &axelar_solana_gas_service::ID,
        &gas_utils.config_authority.pubkey(),
        &gas_utils.config_pda,
        &receiver.pubkey(),
        sol_amount,
    )
    .unwrap();
    // mark that authority does not need to be a signer
    ix.accounts[0].is_signer = false;

    let res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
            ],
        )
        .await;

    assert!(res.is_err());
}
