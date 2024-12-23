use axelar_rkyv_encoding::types::CrossChainId;
use axelar_solana_governance::state::proposal::ExecutableProposal;
use solana_program_test::tokio;
use solana_sdk::{instruction::AccountMeta, signature::Signer};

use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, gmp_sample_metadata,
    ix_builder_with_sample_proposal_data, setup_programs,
};

mod approve_operator;
mod cancel_operator;
mod cancel_time_lock_proposal;
mod schedule_time_lock_proposal;

#[tokio::test]
async fn test_gov_gmp_fails_on_wrong_source_address() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let mut gmp_metadata = gmp_sample_metadata();
    let wrong_address = "0x32Be343B94f860124dC4fEe278FDCBD38C102D88"; // <--- Wrong address
    wrong_address.clone_into(&mut gmp_metadata.source_address);

    let ix_builder = ix_builder_with_sample_proposal_data();

    let mut ix = ix_builder
        .gmp_ix()
        .with_metadata(gmp_metadata.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    approve_ix_at_gateway(&mut sol_integration, &mut ix, gmp_metadata).await;

    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Incoming governance GMP message came with non authorized address: 0x32Be343B94f860124dC4fEe278FDCBD38C102D88");
}

#[tokio::test]
async fn test_gov_gmp_fails_on_wrong_source_chain() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let mut gmp_metadata = gmp_sample_metadata();
    gmp_metadata.cross_chain_id = CrossChainId::new("wrong_chain".to_owned(), "0".to_owned()); // Wrong source chain.

    let ix_builder = ix_builder_with_sample_proposal_data();
    let mut ix = ix_builder
        .gmp_ix()
        .with_metadata(gmp_metadata.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    approve_ix_at_gateway(&mut sol_integration, &mut ix, gmp_metadata).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(
        res,
        "Incoming governance GMP message came with non authorized chain: wrong_chain",
    );
}

#[tokio::test]
async fn test_incoming_proposal_pda_derivation_is_checked_when_receiving_gmp() {
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    let ix_builder = ix_builder_with_sample_proposal_data();

    // We set a wrong address in the payload, then we hash it and derive the PDA,
    // then we send the instruction with the wrong PDA.
    let meta = gmp_sample_metadata();
    let mut ix = ix_builder
        .gmp_ix()
        .with_metadata(meta.clone())
        .schedule_time_lock_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();

    ix.accounts[3] = AccountMeta::new(ExecutableProposal::pda(&[0_u8; 32]).0, false); // Wrong PDA regarding builder data.

    approve_ix_at_gateway(&mut sol_integration, &mut ix, meta).await;
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    // Solana runtime detects the wrong PDA and returns an error.
    assert_msg_present_in_logs(res, "Derived proposal PDA does not match provided one");
}
