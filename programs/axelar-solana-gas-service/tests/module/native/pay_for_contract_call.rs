use axelar_solana_gas_service::events::GasPaidEvent;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use event_cpi_test_utils::assert_event_cpi;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer};

#[tokio::test]
async fn test_pay_native_for_contract_call() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Record balances before the transaction
    let payer = Keypair::new();
    test_fixture
        .fund_account(&payer.pubkey(), 1_000_000_000)
        .await;
    let payer_balance_before = test_fixture
        .try_get_account_no_checks(&payer.pubkey())
        .await
        .unwrap()
        .unwrap()
        .lamports;
    let config_pda_balance_before = test_fixture
        .try_get_account_no_checks(&gas_utils.config_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    // Action
    let refund_address = Pubkey::new_unique();
    let gas_amount = 1_000_000;
    let destination_chain = "ethereum".to_owned();
    let destination_addr = "destination addr 123".to_owned();
    let payload_hash = [42; 32];
    let ix = axelar_solana_gas_service::instructions::pay_native_for_contract_call_instruction(
        &payer.pubkey(),
        destination_chain.clone(),
        destination_addr.clone(),
        payload_hash,
        refund_address,
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
                // pays for gas deduction
                &payer,
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

    let expected_event = GasPaidEvent {
        sender: payer.pubkey(),
        destination_chain: destination_chain.clone(),
        destination_address: destination_addr.clone(),
        payload_hash,
        amount: gas_amount,
        refund_address,
        mint: None,
        token_program_id: None,
        sender_token_account: None,
    };

    assert_event_cpi(&expected_event, &inner_ixs);

    // Execute the transaction
    let _res = test_fixture
        .send_tx_with_custom_signers(
            &[ix],
            &[
                // pays for tx
                &test_fixture.payer.insecure_clone(),
                // pays for gas deduction
                &payer,
            ],
        )
        .await
        .unwrap();

    // assert that SOL gets transferred
    let payer_balance_after = test_fixture
        .try_get_account_no_checks(&payer.pubkey())
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
        config_pda_balance_before + gas_amount
    );
    assert_eq!(payer_balance_after, payer_balance_before - gas_amount);
}

#[tokio::test]
async fn fails_if_payer_not_signer() {
    // Setup
    let pt = ProgramTest::default();
    let mut test_fixture = TestFixture::new(pt).await;
    let gas_utils = test_fixture.deploy_gas_service().await;
    test_fixture.init_gas_config(&gas_utils).await.unwrap();

    // Record balances before the transaction
    let payer = Keypair::new();
    test_fixture
        .fund_account(&payer.pubkey(), 1_000_000_000)
        .await;

    // Action
    let refund_address = Pubkey::new_unique();
    let gas_amount = 1_000_000;
    let destination_chain = "ethereum".to_owned();
    let destination_addr = "destination addr 123".to_owned();
    let payload_hash = [42; 32];
    let mut ix = axelar_solana_gas_service::instructions::pay_native_for_contract_call_instruction(
        &payer.pubkey(),
        destination_chain.clone(),
        destination_addr.clone(),
        payload_hash,
        refund_address,
        gas_amount,
    )
    .unwrap();
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
