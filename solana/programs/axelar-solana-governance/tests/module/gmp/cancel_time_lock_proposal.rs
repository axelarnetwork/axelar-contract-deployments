use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use borsh::to_vec;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

use crate::gmp::gmp_sample_metadata;
use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, events,
    ix_builder_with_sample_proposal_data, setup_programs,
};

#[tokio::test]
async fn test_an_scheduled_proposal_can_be_cancelled() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let ix_builder = ix_builder_with_sample_proposal_data();
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

    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .cancel_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Get the proposal pda and assert it has no data (as it was cancelled)
    let prop_account = sol_integration
        .try_get_account_no_checks(&ix_builder.proposal_pda())
        .await
        .unwrap();
    assert_eq!(prop_account, None);

    // Assert the CancelTimeLockProposal event was emitted.
    let mut emitted_events = events(&res.unwrap());
    assert_eq!(emitted_events.len(), 1);
    let expected_event = cancel_timelock_proposal_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events.pop().unwrap().parse().unwrap();
    assert_eq!(expected_event, got_event);
}

fn cancel_timelock_proposal_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::ProposalCancelled {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: to_vec(&builder.proposal_call_data()).unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
        eta: builder.proposal_u256_le_eta(),
    }
}

#[tokio::test]
async fn test_a_non_existent_scheduled_proposal_cannot_be_cancelled() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let ix_builder = ix_builder_with_sample_proposal_data();
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .cancel_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());

    // Assert no event was emitted.
    let emitted_events = events(&res.clone().err().unwrap());
    assert_eq!(emitted_events.len(), 0);
    assert_msg_present_in_logs(res.err().unwrap(), "Proposal PDA is not initialized");
}

#[tokio::test]
async fn test_program_checks_proposal_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let mut ix_builder = ix_builder_with_sample_proposal_data();
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

    ix_builder.prop_target = Some([1_u8; 32].to_vec().try_into().unwrap());

    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .cancel_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());
    assert_msg_present_in_logs(
        res.err().unwrap(),
        "Derived proposal PDA does not match provided one",
    );
}
