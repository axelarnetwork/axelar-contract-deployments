#![cfg(test)]
use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use axelar_solana_its::state::token_manager::TokenManager;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;

use crate::program_test;

#[rstest::rstest]
#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn test_its_gmp_payload_deploy_token_manager() {
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

    let token_id = Pubkey::new_unique();

    let its_gmp_payload = DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.to_bytes().into(),
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
            &solana_chain.fixture.payer.pubkey(),
            gateway_approved_command_pdas.first().unwrap(),
            &solana_chain.gateway_root_pda,
            message.into(),
            abi_payload,
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

    let (token_manager_pda, _bump) =
        axelar_solana_its::token_manager_pda(&its_root_pda, token_id.as_ref());

    let token_manager = solana_chain
        .fixture
        .get_rkyv_account::<TokenManager>(&token_manager_pda, &axelar_solana_its::id())
        .await;

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
}
