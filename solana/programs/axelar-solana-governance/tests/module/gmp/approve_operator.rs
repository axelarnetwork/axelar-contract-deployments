use axelar_solana_governance::events::GovernanceEvent;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use borsh::to_vec;
use solana_program_test::tokio;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::gmp::gmp_sample_metadata;
use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, events,
    ix_builder_with_sample_proposal_data, setup_programs,
};

#[tokio::test]
async fn test_successfully_process_gmp_approve_operator_proposal() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();

    // We first schedule a time lock proposal
    let msg_meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(msg_meta.clone())
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

    // Assert event was emitted
    let mut emitted_events = events(&res.unwrap());
    assert_eq!(emitted_events.len(), 1);
    let expected_event = operator_proposal_approved_event(&ix_builder);
    let got_event: GovernanceEvent = emitted_events.pop().unwrap().parse().unwrap();
    assert_eq!(expected_event, got_event);
}

fn operator_proposal_approved_event(builder: &IxBuilder<ProposalRelated>) -> GovernanceEvent {
    GovernanceEvent::OperatorProposalApproved {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: to_vec(&builder.proposal_call_data()).unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
    }
}

#[tokio::test]
async fn test_operator_proposal_management_cannot_be_enabled_twice() {
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

    // Enable operator proposal management. First time.
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

    // Enable operator proposal management. Second time. THIS MUST FAIL.
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());
    assert_msg_present_in_logs(
        res.clone().err().unwrap(),
        "Create Account: account Address",
    );
    assert_msg_present_in_logs(res.err().unwrap(), "already in use");
}

#[tokio::test]
async fn test_program_checks_proposal_pda_is_correctly_derived() {
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

    // Second, we try to approve the proposal, but we break the calldata payload, so
    // the hashes don't match with previous PDA derivation. THIS SHOULD FAIL.
    let mut builder = ix_builder.clone();
    builder.prop_target = Some([1_u8; 32].to_vec().try_into().unwrap());
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = builder
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
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

    // Second, we try to approve the proposal, but we break the calldata payload, so
    // the hashes don't match with previous PDA derivation. THIS SHOULD FAIL.
    let builder = ix_builder.clone();
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = builder
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    gmp_call_data.ix.accounts[4] = AccountMeta::new(Pubkey::new_unique(), false); // Wrong PDA regarding builder data.

    approve_ix_at_gateway(&mut sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_err());
    assert_msg_present_in_logs(
        res.err().unwrap(),
        "Derived operator managed proposal PDA does not match provided one",
    );
}
