use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use axelar_solana_governance::state::operator;
use axelar_solana_governance::state::proposal::ExecutableProposal;
use borsh::to_vec;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

use crate::fixtures::MINIMUM_PROPOSAL_DELAY;
use crate::gmp::{assert_msg_present_in_logs, gmp_sample_metadata};
use crate::helpers::{
    approve_ix_at_gateway, events, ix_builder_with_sample_proposal_data, setup_programs,
};

#[tokio::test]
async fn test_successfully_process_gmp_schedule_time_proposal() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();
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

    // Assert account with correct proposal data was created
    let got_proposal = sol_integration
        .fixture
        .get_account::<axelar_solana_governance::state::proposal::ExecutableProposal>(
            &ix_builder.proposal_pda(),
            &axelar_solana_governance::ID,
        )
        .await;

    let bump = ExecutableProposal::pda(&ix_builder.proposal_hash()).1;
    let managed_bump = operator::derive_managed_proposal_pda(&ix_builder.proposal_hash()).1;

    let expected_proposal =
        ExecutableProposal::new(ix_builder.prop_eta.unwrap(), bump, managed_bump);
    assert_eq!(expected_proposal, got_proposal);

    // Assert event was emitted
    let mut emitted_events = events(&res);
    assert_eq!(emitted_events.len(), 1);
    let expected_event = proposal_scheduled_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events.pop().unwrap().parse().unwrap();
    assert_eq!(expected_event, got_event);
}

fn proposal_scheduled_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::ProposalScheduled {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: to_vec(&builder.proposal_call_data()).unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
        eta: builder.proposal_u256_le_eta(),
    }
}

#[tokio::test]
async fn test_time_lock_default_is_enforced() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let mut ix_builder = ix_builder_with_sample_proposal_data();

    // Set artificial, absolute current time
    let now = 1_728_286_884;
    sol_integration.fixture.set_time(i64::from(now)).await;

    // Set an ETA with not enough delay, as default is
    // fixtures::MINIMUM_PROPOSAL_ETA
    ix_builder.prop_eta = Some(u64::from(now) + u64::from(MINIMUM_PROPOSAL_DELAY) - 5);
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

    let got_proposal = sol_integration
        .fixture
        .get_account::<axelar_solana_governance::state::proposal::ExecutableProposal>(
            &ix_builder.proposal_pda(),
            &axelar_solana_governance::ID,
        )
        .await;

    // Assert proposal ETA was overwritten by the default ETA in config
    // MINIMUM_PROPOSAL_DELAY.
    let expected = (now + MINIMUM_PROPOSAL_DELAY) as u64;
    assert_eq!(expected, got_proposal.eta());
}

#[tokio::test]
async fn test_program_checks_proposal_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();
    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    ix.accounts[3] = ix.accounts[2].clone(); // Wrong PDA account
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Derived proposal PDA does not match provided one");
}
