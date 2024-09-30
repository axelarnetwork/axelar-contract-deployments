#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::tests_outside_test_module,
    clippy::str_to_string
)]

use alloy_sol_types::SolValue;
use axelar_rkyv_encoding::types::CrossChainId;
use governance::state::GovernanceConfig;
use governance_gmp::GovernanceCommand;
use helpers::{default_gmp_metadata, evm_governance_payload_command, init_contract, program_test};
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::TestFixture;

mod fixtures;
mod helpers;

#[tokio::test]
async fn test_successfully_initialize_config() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let (config_pda, bump) = GovernanceConfig::pda();

    let config = governance::state::GovernanceConfig::new(bump, [0_u8; 32], [0_u8; 32]);

    // Action
    let ix =
        governance::instructions::initialize_config(&fixture.payer.pubkey(), &config, &config_pda)
            .unwrap();
    let res = fixture.send_tx_with_metadata(&[ix]).await;

    // Assert
    assert!(res.result.is_ok());
    let root_pda_data = fixture
        .get_rkyv_account::<governance::state::GovernanceConfig>(&config_pda, &governance::ID)
        .await;
    assert_eq!(&config, &root_pda_data);
}

#[tokio::test]
async fn test_successfully_receives_gmp_command() {
    let mut fixture = TestFixture::new(program_test()).await;

    // Setup (initialize contract)
    let (config_pda, _) = init_contract(&mut fixture).await.unwrap();

    // Action (Send GMP message)
    let governance_instruction =
        governance::instructions::GovernanceInstruction::GovernanceGmpPayload {
            payload: evm_governance_payload_command(GovernanceCommand::ApproveOperatorProposal)
                .abi_encode(),
            metadata: default_gmp_metadata(),
        };

    let ix = governance::instructions::send_gmp_governance_message(
        &fixture.payer.pubkey(),
        &config_pda,
        &governance_instruction,
    )
    .unwrap();
    let result = fixture.send_tx_with_metadata(&[ix]).await;
    assert!(result.result.is_ok());

    // Assert
    // todo check events emitted whenever implemented.
    // currently we should see a line like:
    // [2024-09-23T20:20:36.120940638Z DEBUG
    // solana_runtime::message_processor::stable_log] Program log: Executing
    // ApproveOperatorProposal !
}

#[tokio::test]
async fn test_gov_gmp_fails_on_wrong_source_address() {
    let mut fixture = TestFixture::new(program_test()).await;

    // Setup (initialize contract)
    let (config_pda, _) = init_contract(&mut fixture).await.unwrap();

    let mut gmp_metadata = default_gmp_metadata();
    let wrong_address = "0x32Be343B94f860124dC4fEe278FDCBD38C102D88"; // <--- Wrong address
    wrong_address.clone_into(&mut gmp_metadata.source_address);

    // Action (Send GMP message)
    let governance_instruction =
        governance::instructions::GovernanceInstruction::GovernanceGmpPayload {
            payload: evm_governance_payload_command(GovernanceCommand::ApproveOperatorProposal)
                .abi_encode(),
            metadata: gmp_metadata,
        };

    let ix = governance::instructions::send_gmp_governance_message(
        &fixture.payer.pubkey(),
        &config_pda,
        &governance_instruction,
    )
    .unwrap();
    let res = fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(res, "Incoming governance GMP message came with non authorized address: 0x32Be343B94f860124dC4fEe278FDCBD38C102D88");
}

#[tokio::test]
async fn test_gov_gmp_fails_on_wrong_source_chain() {
    let mut fixture = TestFixture::new(program_test()).await;

    // Setup (initialize contract)
    let (config_pda, _) = init_contract(&mut fixture).await.unwrap();

    let mut gmp_metadata = default_gmp_metadata();
    gmp_metadata.cross_chain_id = CrossChainId::new("wrong_chain".to_owned(), "0".to_owned()); // Wrong source chain.

    // Action (Send GMP message)
    let governance_instruction =
        governance::instructions::GovernanceInstruction::GovernanceGmpPayload {
            payload: evm_governance_payload_command(GovernanceCommand::ApproveOperatorProposal)
                .abi_encode(),
            metadata: gmp_metadata,
        };

    let ix = governance::instructions::send_gmp_governance_message(
        &fixture.payer.pubkey(),
        &config_pda,
        &governance_instruction,
    )
    .unwrap();
    let res = fixture.send_tx_with_metadata(&[ix]).await;
    assert!(res.result.is_err());
    assert_msg_present_in_logs(
        res,
        "Incoming governance GMP message came with non authorized chain: wrong_chain",
    );
}

fn assert_msg_present_in_logs(res: BanksTransactionResultWithMetadata, msg: &str) {
    assert!(
        res.metadata
            .unwrap()
            .log_messages
            .into_iter()
            .any(|x| x.contains(msg)),
        "Expected error message not found!"
    );
}
