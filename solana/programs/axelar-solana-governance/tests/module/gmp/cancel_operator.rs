use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use borsh::to_vec;
use solana_program_test::tokio;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::gmp::{gmp_sample_metadata, setup_programs};
use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, events, ix_builder_with_sample_proposal_data,
};

#[tokio::test]
async fn test_successfully_process_gmp_cancel_operator_proposal() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();

    // We first schedule a time lock proposal
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

    // Second, we approve the proposal, so the operator should be able to execute it
    // regardless of the ETA.
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Assert account with correct marker data data was created
    let approved_operator = sol_integration
        .fixture
        .banks_client
        .get_account(ix_builder.proposal_operator_marker_pda())
        .await
        .unwrap();

    assert!(approved_operator.is_some());

    // Third, we cancel the operator proposal
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .cancel_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Assert the account and its data was deleted
    let acc = sol_integration
        .fixture
        .banks_client
        .get_account(ix_builder.proposal_operator_marker_pda())
        .await
        .unwrap();
    assert_eq!(acc, None);

    // Assert event was emitted
    let mut emitted_events = events(&res.unwrap());
    assert_eq!(emitted_events.len(), 1);
    let expected_event = operator_proposal_cancelled_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events.pop().unwrap().parse().unwrap();
    assert_eq!(expected_event, got_event);
}

fn operator_proposal_cancelled_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::OperatorProposalCancelled {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: to_vec(&builder.proposal_call_data()).unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
    }
}

#[tokio::test]
async fn test_program_checks_proposal_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let mut ix_builder = ix_builder_with_sample_proposal_data();

    // We first schedule a time lock proposal
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

    // Second, we approve the operator management of the proposal
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Third, we try to cancel the operator management of the proposal
    let meta = gmp_sample_metadata();
    ix_builder.prop_target = Some([1_u8; 32].to_vec().try_into().unwrap());

    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .cancel_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());

    assert_msg_present_in_logs(
        res.err().unwrap(),
        "Derived proposal PDA does not match provided one",
    );
}

#[tokio::test]
async fn test_program_checks_operator_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();

    // We first schedule a time lock proposal
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

    // Second, we approve the operator management of the proposal
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Third, we try to cancel the operator management of the proposal
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .cancel_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    gmp_call_data.ix.accounts[4] = AccountMeta::new_readonly(Pubkey::new_unique(), false); // Wrong PDA account
    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());

    assert_msg_present_in_logs(
        res.err().unwrap(),
        "Derived operator managed proposal PDA does not match provided one",
    );
}
