use axelar_solana_governance::instructions::builder::IxBuilder;
use solana_program_test::tokio;
use solana_sdk::account::WritableAccount;
use solana_sdk::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_sdk::signature::{Keypair, Signer};
use test_fixtures::test_setup::add_upgradeable_loader_account;

use crate::helpers::{
    approve_ix_at_gateway, assert_msg_present_in_logs, default_proposal_eta, gmp_memo_metadata,
    setup_programs,
};
#[tokio::test]
async fn test_gateway_upgrade_through_proposal() {
    // Init environment
    let (mut sol_integration, config_pda, _) = setup_programs().await;

    // Upload bytecode of the new gateway version
    let new_gateway_version = tokio::fs::read("../../target/deploy/dummy_axelar_solana_gateway.so")
        .await
        .unwrap();
    let buffer_address = Keypair::new();
    let programdata_data_offset = UpgradeableLoaderState::size_of_buffer_metadata();

    add_upgradeable_loader_account(
        &mut sol_integration.fixture.context,
        &buffer_address.pubkey(),
        &UpgradeableLoaderState::Buffer {
            authority_address: Some(config_pda),
        },
        UpgradeableLoaderState::size_of_buffer(new_gateway_version.len()),
        |account| {
            account.data_as_mut_slice()[programdata_data_offset..]
                .copy_from_slice(&new_gateway_version);
        },
    )
    .await;

    // Send the upgrade proposal with the new buffer account
    let ix_builder = IxBuilder::builder_for_program_upgrade(
        &gateway::ID,
        &buffer_address.pubkey(),
        &config_pda,
        &sol_integration.fixture.payer.pubkey(),
        default_proposal_eta(),
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

    // Advance time
    sol_integration
        .fixture
        .forward_time(default_proposal_eta() as i64)
        .await;

    // Execute the proposal
    let ix = ix_builder
        .clone()
        .execute_proposal(&sol_integration.fixture.payer.pubkey(), &config_pda)
        .build();
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());

    // Advance slot to the next slot
    sol_integration.fixture.context.warp_to_slot(2).unwrap();

    // Now we can send ixs to the new program
    let ix = dummy_axelar_solana_gateway::instructions::echo(
        gateway::ID,
        "Testing gateway upgrade".to_string(),
    );
    let res = sol_integration.fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_ok());
    assert_msg_present_in_logs(res, "Echo: Testing gateway upgrade");
}
