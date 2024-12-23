use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use rkyv::Deserialize;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

use crate::gmp::gmp_sample_metadata;
use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, events,
    ix_builder_with_sample_proposal_data, setup_programs,
};

#[tokio::test]
async fn test_an_scheduled_proposal_can_be_cancelled() {
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

    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .cancel_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Get the proposal pda and assert it has no data (as it was cancelled)
    let prop_account = sol_integration
        .fixture
        .banks_client
        .get_account(ix_builder.proposal_pda())
        .await
        .unwrap();
    assert_eq!(prop_account, None);

    // Assert the CancelTimeLockProposal event was emitted.
    let mut emitted_events = events(&res);
    assert_eq!(emitted_events.len(), 1);
    let expected_event = cancel_timelock_proposal_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events
        .pop()
        .unwrap()
        .parse()
        .deserialize(&mut rkyv::Infallible)
        .unwrap();
    assert_eq!(expected_event, got_event);
}

fn cancel_timelock_proposal_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::ProposalCancelled {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: builder.proposal_call_data().to_bytes().unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
        eta: builder.proposal_u256_le_eta(),
    }
}

#[tokio::test]
async fn test_a_non_existent_scheduled_proposal_cannot_be_cancelled() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();
    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .cancel_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());

    // Assert no event was emitted.
    let emitted_events = events(&res);
    assert_eq!(emitted_events.len(), 0);
    assert_msg_present_in_logs(res, "Proposal PDA is not initialized");
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
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .clone()
        .gmp_ix()
        .with_metadata(meta.clone())
        .cancel_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    ix.accounts[3] = ix.accounts[2].clone(); // Wrong PDA account
    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Derived proposal PDA does not match provided one");
}
