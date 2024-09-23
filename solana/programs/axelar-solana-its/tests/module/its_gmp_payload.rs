#![cfg(test)]
use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

use crate::program_test;

#[rstest::rstest]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload() {
    let mut solana_chain = program_test().await;
    let (its_root_pda, its_root_pda_bump) =
        axelar_solana_its::its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(its_root_pda, its_root_pda_bump),
        )
        .unwrap()])
        .await;

    let its_gmp_payload = DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: [0_u8; 32].into(),
        token_manager_type: alloy_primitives::Uint::<256, 4>::from(0_u128),
        params: vec![].into(),
    };
    let abi_payload = GMPPayload::DeployTokenManager(its_gmp_payload).encode();
    let payload_hash = solana_sdk::keccak::hash(&abi_payload).to_bytes();
    let message = random_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );
    // Action: "Relayer" calls Gateway to approve messages
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![message.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let tx = solana_chain
        .fixture
        .send_tx_with_metadata(&[axelar_solana_its::instructions::its_gmp_payload(
            gateway_approved_command_pdas.first().unwrap(),
            &solana_chain.gateway_root_pda,
            abi_payload,
            message.cc_id().clone(),
            message.source_address().to_owned(),
            message.destination_address().to_owned(),
            "solana-devnet".to_owned(),
        )
        .unwrap()])
        .await;

    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs
            .iter()
            .any(|log| log.as_str().contains("Received DeployTokenManager message")),
        "expected ITS call log not found in logs"
    );
}
