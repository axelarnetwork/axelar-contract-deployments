use governance::events::GovernanceEvent;
use governance::instructions::builder::{IxBuilder, ProposalRelated};
use rkyv::Deserialize;
use solana_program_test::tokio;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::signature::Signer;

use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, default_proposal_eta, events,
    gmp_memo_metadata, gmp_sample_metadata, ix_builder_with_memo_proposal_data,
    ix_builder_with_sample_proposal_data, setup_programs,
};

#[tokio::test]
async fn test_time_lock_is_enforced() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let mut ix_builder = ix_builder_with_sample_proposal_data();

    // Set artificial, absolute current time
    let now = 1_728_286_884;
    sol_integration
        .fixture
        .set_time(i64::try_from(now).unwrap())
        .await;

    // We should not be able to execute the proposal yet, as eta is 10 seconds
    // ahead.
    let eta: u64 = now + 10;

    // Get default fixtures
    ix_builder.prop_eta = Some(eta);
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

    // Send execute proposal instruction
    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Proposal ETA needs to be respected");
}

#[tokio::test]
async fn test_proposal_can_be_executed_and_reached_memo_program() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    // Memo program solana accounts. gathered from
    // `axelar_solana_memo_program::instruction::call_gateway_with_memo`

    let memo_program_accounts = &[
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Second flow, execute the proposal.

    // Advance time so we can execute PDA
    sol_integration
        .fixture
        .set_time(default_proposal_eta() as i64)
        .await;

    // Send execute proposal instruction
    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Assert event was emitted
    let mut emitted_events = events(&res);
    assert_eq!(emitted_events.len(), 1);
    let expected_event = proposal_executed_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events
        .pop()
        .unwrap()
        .parse()
        .deserialize(&mut rkyv::Infallible)
        .unwrap();
    assert_eq!(expected_event, got_event);
    assert_msg_present_in_logs(res, "Instruction: SendToGateway");
}

fn proposal_executed_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::ProposalExecuted {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: builder.proposal_call_data().to_bytes().unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
        eta: builder.proposal_u256_le_eta(),
    }
}

#[tokio::test]
async fn test_execution_of_proposal_cannot_be_done_facilitating_unaligned_pda_regarding_payload() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let mut ix_builder = ix_builder_with_sample_proposal_data();

    // We send a legit proposal
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

    // We send a wrong execution proposal instruction, with a wrong PDA.

    ix_builder.prop_target = Some([0_u8; 32].to_vec().try_into().unwrap());

    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    // The runtime detects the wrong PDA and returns an error.
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Derived proposal PDA does not match provided one");
}

#[tokio::test]
async fn test_proposal_can_be_executed_and_reached_memo_program_transferring_funds() {
    let (mut sol_integration, config_pda, counter_pda) = setup_programs().await;

    // Fund the governance PDA
    let ix = solana_sdk::system_instruction::transfer(
        &sol_integration.fixture.payer.pubkey(),
        &config_pda,
        LAMPORTS_PER_SOL, // Let's fund the governance PDA with 1 SOL.
    );
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Gmp send the memo program instruction.
    // Memo program solana accounts. gathered from
    // `axelar_solana_memo_program::instruction::call_gateway_with_memo`

    let memo_program_accounts = &[
        AccountMeta::new(sol_integration.fixture.payer.pubkey(), true),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
    ];
    let memo_program_funds_receiver_account = AccountMeta::new(counter_pda, false);
    let ix_builder = ix_builder_with_memo_proposal_data(
        memo_program_accounts,
        LAMPORTS_PER_SOL,
        Some(memo_program_funds_receiver_account.clone()),
    );
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Second flow, execute the proposal.

    // Advance time so we can execute PDA
    sol_integration
        .fixture
        .set_time(default_proposal_eta() as i64)
        .await;

    // Send execute proposal instruction

    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());
    assert_msg_present_in_logs(res, "Instruction: SendToGateway");

    let target_contract_balance = sol_integration
        .fixture
        .banks_client
        .get_balance(counter_pda)
        .await
        .unwrap();

    assert_eq!(LAMPORTS_PER_SOL + 953_520, target_contract_balance);
}

#[tokio::test]
async fn test_proposal_is_deleted_after_execution() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    // Memo program solana accounts. gathered from
    // `axelar_solana_memo_program::instruction::call_gateway_with_memo`

    let memo_program_accounts = &[
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Second flow, execute the proposal.

    // Advance time so we can execute PDA
    sol_integration
        .fixture
        .set_time(default_proposal_eta() as i64)
        .await;

    // Send execute proposal instruction

    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Proposal should be deleted
    let proposal_account = sol_integration
        .fixture
        .banks_client
        .get_account(ix_builder.proposal_pda())
        .await
        .unwrap();
    assert!(proposal_account.is_none());
}

#[tokio::test]
async fn test_same_proposal_can_be_created_after_execution() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    // Memo program solana accounts. gathered from
    // `axelar_solana_memo_program::instruction::call_gateway_with_memo`

    let memo_program_accounts = &[
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Second flow, execute the proposal.

    // Advance time so we can execute PDA
    sol_integration
        .fixture
        .set_time(default_proposal_eta() as i64)
        .await;

    // Send execute proposal instruction
    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Try to create again the proposal, it should be possible.
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());
}

#[tokio::test()]
async fn test_cannot_create_proposal_twice() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    // Memo program solana accounts. gathered from
    // `axelar_solana_memo_program::instruction::call_gateway_with_memo`

    let memo_program_accounts = &[
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);

    // Get memo gmp fixtures
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Try to create again the proposal, it should fail.
    let meta = gmp_memo_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());

    // We split the error message in two, as the error message contains addresses
    // that are changing in each test run.
    assert_msg_present_in_logs(res.clone(), "Create Account: account Address"); 
    assert_msg_present_in_logs(res, "already in use");
}
