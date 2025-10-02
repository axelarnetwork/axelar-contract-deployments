use axelar_solana_gas_service::events::NativeGasRefundedEvent;
use axelar_solana_gateway_test_fixtures::{assert_msg_present_in_logs, base::TestFixture};
use event_cpi_test_utils::assert_event_cpi;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_refund_native() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Record balances before the transaction
    test_fixture
        .fund_account(&gas_utils.config_pda, 1_000_000_000)
        .await;
    let refunded_user = Keypair::new();
    let refunder_balance_before = 0;
    let config_pda_balance_before = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Action
    let gas_amount = 1_000_000;
    let tx_hash = [42; 64];
    let ix_index = 1;
    let event_ix_index = 2;
    let ix = axelar_solana_gas_service::instructions::refund_native_fees_instruction(
        &gas_utils.operator.pubkey(),
        &refunded_user.pubkey(),
        tx_hash,
        ix_index,
        event_ix_index,
        gas_amount,
    )
    .unwrap();

    // First simulate to check events
    let simulation_result = test_fixture
        .simulate_tx_with_custom_signers(
            &[ix.clone()],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
                // operator for config pda deduction
                &gas_utils.operator,
            ],
        )
        .await
        .unwrap();

    // Assert event emitted
    let inner_ixs = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    assert!(!inner_ixs.is_empty());

    let expected_event = NativeGasRefundedEvent {
        config_pda: gas_utils.config_pda,
        tx_hash,
        ix_index,
        event_ix_index,
        receiver: refunded_user.pubkey(),
        fees: gas_amount,
    };

    assert_event_cpi(&expected_event, &inner_ixs);

    // Execute the transaction
    let _res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
                // operator for config pda deduction
                &gas_utils.operator,
            ],
        )
        .await
        .unwrap();

    // assert that SOL gets transferred
    let refunder_balance_after = test_fixture
        .try_get_account_no_checks(&refunded_user.pubkey())
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
        config_pda_balance_before - gas_amount
    );
    assert_eq!(refunder_balance_after, refunder_balance_before + gas_amount);
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
    let refunded_user = Keypair::new();
    let gas_amount = 1_000_000;
    let tx_hash = [42; 64];
    let ix_index = 1;
    let event_ix_index = 2;
    let mut ix = axelar_solana_gas_service::instructions::refund_native_fees_instruction(
        &gas_utils.operator.pubkey(),
        &refunded_user.pubkey(),
        tx_hash,
        ix_index,
        event_ix_index,
        gas_amount,
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
                // note -- missing authortiy signature here
            ],
        )
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_refund_native_fails_with_zero_fee() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();
    test_fixture
        .fund_account(&gas_utils.config_pda, 1_000_000_000)
        .await;

    // Action - attempt to refund with zero fee
    let refunded_user = Keypair::new();
    let gas_amount = 0; // Zero fee should fail
    let tx_hash = [42; 64];
    let ix_index = 1;
    let event_ix_index = 2;
    let ix = axelar_solana_gas_service::instructions::refund_native_fees_instruction(
        &gas_utils.operator.pubkey(),
        &refunded_user.pubkey(),
        tx_hash,
        ix_index,
        event_ix_index,
        gas_amount,
    )
    .unwrap();

    let res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
                // operator for config pda deduction
                &gas_utils.operator,
            ],
        )
        .await;

    // Assert that the transaction fails due to zero fee
    assert!(res.is_err());
    assert_msg_present_in_logs(res.unwrap_err(), "Gas fee amount cannot be zero");
}
