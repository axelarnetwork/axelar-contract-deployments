use governance::instructions::builder::IxBuilder;
use solana_program_test::tokio;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;

use crate::helpers::{
    approve_ix_at_gateway, default_proposal_eta, gmp_sample_metadata, setup_programs,
};

#[tokio::test]
async fn test_can_withdraw_native_tokens_from_contract() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;
    let fund_receiver = Keypair::new();

    // Fund both accounts for avoiding rent exemption issues.
    let ix = system_instruction::transfer(
        &sol_integration.fixture.payer.pubkey(),
        &config_pda,
        LAMPORTS_PER_SOL,
    );
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());
    let ix = system_instruction::transfer(
        &sol_integration.fixture.payer.pubkey(),
        &fund_receiver.pubkey(),
        LAMPORTS_PER_SOL,
    );
    let amount_to_withdraw = 1;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    let ix_builder = IxBuilder::new().builder_for_withdraw_tokens(
        &config_pda,
        &fund_receiver.pubkey(),
        amount_to_withdraw,
        default_proposal_eta(),
    );

    // Send the GMP instruction
    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());
    let initial_governance_pda_funds = sol_integration
        .fixture
        .banks_client
        .get_balance(config_pda)
        .await
        .unwrap();
    println!("initial_governance_pda_funds: {initial_governance_pda_funds}");

    // Move time forward to the proposal ETA
    sol_integration
        .fixture
        .set_time(default_proposal_eta() as i64)
        .await;

    // Get current receiver total funds
    let initial_receiver_funds = sol_integration
        .fixture
        .banks_client
        .get_balance(fund_receiver.pubkey())
        .await
        .unwrap();

    // Get current contract total funds
    let initial_governance_pda_funds = sol_integration
        .fixture
        .banks_client
        .get_balance(config_pda)
        .await
        .unwrap();

    println!("initial_receiver_funds: {initial_receiver_funds}");
    println!("initial_governance_pda_funds: {initial_governance_pda_funds}");

    // Send the proposal execution instruction
    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    println!("{res:?}");
    assert!(res.result.is_ok());

    // Assert the contract has less funds
    let post_withdraw_governance_pda_funds = sol_integration
        .fixture
        .banks_client
        .get_balance(config_pda)
        .await
        .unwrap();

    assert_eq!(
        post_withdraw_governance_pda_funds,
        initial_governance_pda_funds - amount_to_withdraw
    );

    // Assert the receiver has the initial funds + the gov module funds
    let new_receiver_funds = sol_integration
        .fixture
        .banks_client
        .get_balance(fund_receiver.pubkey())
        .await
        .unwrap();
    assert_eq!(
        new_receiver_funds,
        amount_to_withdraw + initial_receiver_funds
    );
}
