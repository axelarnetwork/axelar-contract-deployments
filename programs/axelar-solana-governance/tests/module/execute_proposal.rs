use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use borsh::to_vec;
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
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

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
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Send execute proposal instruction
    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_err());
    assert_msg_present_in_logs(res.err().unwrap(), "Proposal ETA needs to be respected");
}

#[tokio::test]
async fn test_proposal_can_be_executed_and_reached_memo_program() {
    let (mut sol_integration, config_pda, counter_pda) = Box::pin(setup_programs()).await;

    let (memo_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_memo_program::ID);
    // Using the memo program as target proposal program.
    let memo_program_accounts = &[
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
        AccountMeta::new_readonly(counter_pda, false),
        AccountMeta::new_readonly(memo_signing_pda, false),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

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

    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());

    // Assert event was emitted
    let mut emitted_events = events(&res.clone().unwrap());
    assert_eq!(emitted_events.len(), 1);
    let expected_event = proposal_executed_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events.pop().unwrap().parse().unwrap();
    assert_eq!(expected_event, got_event);
    assert_msg_present_in_logs(res.unwrap(), "Instruction: SendToGateway");
}

fn proposal_executed_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::ProposalExecuted {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: to_vec(&builder.proposal_call_data()).unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
        eta: builder.proposal_u256_le_eta(),
    }
}

#[tokio::test]
async fn test_program_checks_proposal_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let mut ix_builder = ix_builder_with_sample_proposal_data();

    // We send a legit proposal
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // We send a wrong execution proposal instruction, with a wrong PDA.

    ix_builder.prop_target = Some([1_u8; 32].to_vec().try_into().unwrap());

    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    let res = sol_integration.fixture.send_tx(&[ix]).await;
    // The runtime detects the wrong PDA and returns an error.
    assert!(res.is_err());

    let meta = res.err().unwrap();

    assert!(meta
        .find_at_least_one_log(&[
            "Derived proposal PDA does not match provided one",
            "Provided seeds do not result in a valid address",
        ])
        .is_some());
}

#[tokio::test]
async fn test_proposal_can_be_executed_and_reached_memo_program_transferring_funds() {
    let (mut sol_integration, config_pda, counter_pda) = Box::pin(setup_programs()).await;

    // Fund the governance PDA
    let ix = solana_sdk::system_instruction::transfer(
        &sol_integration.fixture.payer.pubkey(),
        &config_pda,
        LAMPORTS_PER_SOL, // Let's fund the governance PDA with 1 SOL.
    );
    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());

    // Gmp send the memo program instruction.

    // Using the memo program as target proposal program.
    let memo_program_funds_receiver_account = AccountMeta::new(counter_pda, false);
    let (memo_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_memo_program::ID);

    let memo_program_accounts = &[
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
        memo_program_funds_receiver_account.clone(),
        AccountMeta::new_readonly(memo_signing_pda, false),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
    ];

    let ix_builder: IxBuilder<ProposalRelated> = ix_builder_with_memo_proposal_data(
        memo_program_accounts,
        LAMPORTS_PER_SOL,
        Some(memo_program_funds_receiver_account),
    );
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

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

    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());
    assert_msg_present_in_logs(res.unwrap(), "Instruction: SendToGateway");

    let target_contract_balance = sol_integration.get_balance(&counter_pda).await;

    assert_eq!(LAMPORTS_PER_SOL + 953_520, target_contract_balance);
}

#[tokio::test]
async fn test_proposal_is_deleted_after_execution() {
    let (mut sol_integration, config_pda, counter_pda) = Box::pin(setup_programs()).await;

    // Memo program solana accounts. gathered from
    // `axelar_solana_memo_program_old::instruction::call_gateway_with_memo`

    let (memo_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_memo_program::ID);
    // Using the memo program as target proposal program.
    let memo_program_accounts = &[
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
        AccountMeta::new_readonly(counter_pda, false),
        AccountMeta::new_readonly(memo_signing_pda, false),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

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

    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());

    // Proposal should be deleted
    let proposal_account = sol_integration
        .try_get_account_no_checks(&ix_builder.proposal_pda())
        .await
        .unwrap();
    assert!(proposal_account.is_none());
}

#[tokio::test]
async fn test_same_proposal_can_be_created_after_execution() {
    let (mut sol_integration, config_pda, counter_pda) = Box::pin(setup_programs()).await;

    let (memo_signing_pda, _) =
        axelar_solana_gateway::get_call_contract_signing_pda(axelar_solana_memo_program::ID);
    // Using the memo program as target proposal program.
    let memo_program_accounts = &[
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
        AccountMeta::new_readonly(counter_pda, false),
        AccountMeta::new_readonly(memo_signing_pda, false),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

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

    let res = sol_integration.fixture.send_tx(&[ix]).await;
    assert!(res.is_ok());

    // Try to create again the proposal, it should be possible.
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());
}

#[tokio::test()]
async fn test_cannot_create_proposal_twice() {
    let (mut sol_integration, config_pda, counter_pda) = Box::pin(setup_programs()).await;

    // Using the memo program as target proposal program.
    let memo_program_accounts = &[
        AccountMeta::new_readonly(counter_pda, false),
        AccountMeta::new_readonly(sol_integration.gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
        AccountMeta::new_readonly(axelar_solana_memo_program::id(), false),
        AccountMeta::new_readonly(sol_integration.fixture.payer.pubkey(), true),
    ];

    let ix_builder = ix_builder_with_memo_proposal_data(memo_program_accounts, 0, None);

    // Get memo gmp fixtures
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Try to create again the proposal, it should fail.
    let meta = gmp_memo_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());

    // We split the error message in two, as the error message contains addresses
    // that are changing in each test run.
    assert_msg_present_in_logs(
        res.clone().err().unwrap(),
        "Create Account: account Address",
    );
    assert_msg_present_in_logs(res.err().unwrap(), "already in use");
}
