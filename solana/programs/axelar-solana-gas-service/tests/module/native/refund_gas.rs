use axelar_solana_gas_service::processor::{GasServiceEvent, NativeGasRefundedEvent};
use axelar_solana_gateway_test_fixtures::{base::TestFixture, gas_service::get_gas_service_events};
use gateway_event_stack::ProgramInvocationState;
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
    let log_index = 1232;
    let ix = axelar_solana_gas_service::instructions::refund_native_fees_instruction(
        &axelar_solana_gas_service::ID,
        &gas_utils.config_authority.pubkey(),
        &refunded_user.pubkey(),
        &gas_utils.config_pda,
        tx_hash,
        log_index,
        gas_amount,
    )
    .unwrap();

    let res = test_fixture
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

    // assert event
    let emitted_events = get_gas_service_events(&res).into_iter().next().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GasServiceEvent::NativeGasRefunded(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    assert_eq!(
        emitted_event,
        &NativeGasRefundedEvent {
            config_pda: gas_utils.config_pda,
            tx_hash,
            log_index,
            receiver: refunded_user.pubkey(),
            fees: gas_amount
        }
    );

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
    let log_index = 1232;
    let mut ix = axelar_solana_gas_service::instructions::refund_native_fees_instruction(
        &axelar_solana_gas_service::ID,
        &gas_utils.config_authority.pubkey(),
        &refunded_user.pubkey(),
        &gas_utils.config_pda,
        tx_hash,
        log_index,
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
