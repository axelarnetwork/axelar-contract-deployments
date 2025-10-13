use axelar_solana_gateway_test_fixtures::base::FindLog;
use axelar_solana_gateway_test_fixtures::{
    assert_msg_present_in_logs, SolanaAxelarIntegrationMetadata,
};
use axelar_solana_governance::events;
use axelar_solana_governance::instructions::builder::{IxBuilder, ProposalRelated};
use borsh::to_vec;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::gmp::gmp_sample_metadata;
use crate::helpers::{
    approve_ix_at_gateway, find_first_cpi_event_unchecked, ix_builder_with_sample_proposal_data,
    setup_programs,
};

#[tokio::test]
async fn test_successfully_process_gmp_approve_operator_proposal() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

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
    let simulation_event = find_first_cpi_event_unchecked::<events::OperatorProposalApproved>(
        &mut sol_integration,
        &gmp_call_data.ix,
    )
    .await
    .unwrap();
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());

    // Assert account with correct marker data data was created
    let approved_operator = sol_integration
        .try_get_account_no_checks(&ix_builder.proposal_operator_marker_pda())
        .await
        .unwrap();

    assert!(approved_operator.is_some());

    // Assert correct event was emitted
    let expected_event = operator_proposal_approved_event(&ix_builder);
    assert_eq!(expected_event, simulation_event);
}

fn operator_proposal_approved_event(
    builder: &IxBuilder<ProposalRelated>,
) -> events::OperatorProposalApproved {
    events::OperatorProposalApproved {
        hash: builder.proposal_hash(),
        target_address: builder.proposal_target_address().to_bytes(),
        call_data: to_vec(&builder.proposal_call_data()).unwrap(),
        native_value: builder.proposal_u256_le_native_value(),
    }
}

#[tokio::test]
async fn test_operator_proposal_management_cannot_be_enabled_twice() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

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
        res.err().unwrap(),
        "Proposal already under operator control",
    );
}

#[tokio::test]
async fn test_program_checks_proposal_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let ix_builder = ix_builder_with_sample_proposal_data();

    // We first schedule a time lock proposal
    schedule_time_lock_proposal(&mut sol_integration, &ix_builder, &config_pda).await;

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

    let meta = res.err().unwrap();

    assert!(meta
        .find_at_least_one_log(&[
            "Derived proposal PDA does not match provided one",
            "Provided seeds do not result in a valid address",
        ])
        .is_some());
}

#[tokio::test]
async fn test_program_checks_operator_pda_is_correctly_derived() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

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

#[tokio::test]
async fn test_approval_requires_proposal_pda_to_be_funded() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let ix_builder = ix_builder_with_sample_proposal_data();
    let proposal_pda = ix_builder.proposal_pda();

    schedule_time_lock_proposal(&mut sol_integration, &ix_builder, &config_pda).await;

    // Then defund the account.
    assert_account_is_funded(&mut sol_integration, &proposal_pda).await;
    let mut raw_account = sol_integration
        .try_get_account_no_checks(&proposal_pda)
        .await
        .unwrap()
        .unwrap();
    raw_account.lamports = 0;
    sol_integration.set_account_state(&proposal_pda, raw_account);
    assert_account_has_no_funds(&mut sol_integration, &proposal_pda).await;

    // Attempting to approve a proposal without scheduling it first should fail.
    let res = approve(&mut sol_integration, &ix_builder, &config_pda).await;
    assert!(res.is_err());

    let err = res.err().unwrap();
    assert_msg_present_in_logs(
        err,
        "Failed to load proposal for checking bumps: InsufficientFunds",
    );
}

#[tokio::test]
async fn test_approval_requires_proposal_pda_owner_to_be_governance() {
    let (mut sol_integration, config_pda, _) = Box::pin(setup_programs()).await;

    let ix_builder = ix_builder_with_sample_proposal_data();
    let proposal_pda = ix_builder.proposal_pda();

    // We first schedule a time lock proposal
    schedule_time_lock_proposal(&mut sol_integration, &ix_builder, &config_pda).await;

    // Then transfer the account to a random key instead of the governance program.
    let mut raw_account = sol_integration
        .try_get_account_no_checks(&proposal_pda)
        .await
        .unwrap()
        .unwrap();
    raw_account.owner = Pubkey::new_unique();
    sol_integration.set_account_state(&proposal_pda, raw_account);

    // Attempting to approve a proposal without scheduling it first should fail.
    let res = approve(&mut sol_integration, &ix_builder, &config_pda).await;
    assert!(res.is_err());

    let err = res.err().unwrap();
    assert_msg_present_in_logs(
        err,
        "Failed to load proposal for checking bumps: IllegalOwner",
    );
}

async fn assert_account_has_no_funds(
    sol_integration: &mut SolanaAxelarIntegrationMetadata,
    account: &Pubkey,
) {
    let account_option = sol_integration
        .try_get_account_no_checks(account)
        .await
        .expect("Should not error when checking account");
    assert!(
        account_option.is_none(),
        "Account should not have funds at this point"
    );
}

async fn assert_account_is_funded(
    sol_integration: &mut SolanaAxelarIntegrationMetadata,
    account: &Pubkey,
) {
    let account_option = sol_integration
        .try_get_account_no_checks(account)
        .await
        .expect("Should not error when checking account");
    assert!(
        account_option.is_some(),
        "Account should have funds at this point"
    );
}

async fn approve(
    sol_integration: &mut SolanaAxelarIntegrationMetadata,
    ix_builder: &IxBuilder<ProposalRelated>,
    config_pda: &Pubkey,
) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .approve_operator_proposal(&sol_integration.fixture.payer.pubkey(), config_pda)
        .build();

    approve_ix_at_gateway(sol_integration, &mut gmp_call_data).await;
    sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await
}

async fn schedule_time_lock_proposal(
    sol_integration: &mut SolanaAxelarIntegrationMetadata,
    ix_builder: &IxBuilder<ProposalRelated>,
    config_pda: &Pubkey,
) {
    let meta = gmp_sample_metadata();
    let mut gmp_call_data = ix_builder
        .clone()
        .gmp_ix()
        .with_msg_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), config_pda)
        .build();
    approve_ix_at_gateway(sol_integration, &mut gmp_call_data).await;
    let res = sol_integration.fixture.send_tx(&[gmp_call_data.ix]).await;
    assert!(res.is_ok());
}
