use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use evm_contracts_test_suite::ethers::types::U64;
use evm_contracts_test_suite::ethers::utils::hex;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_gateway::ContractCallFilter;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

use crate::{axelar_evm_setup, axelar_solana_setup, ItsProgramWrapper};

#[tokio::test]
async fn test_send_from_evm_to_solana() {
    // Setup - Solana
    let ItsProgramWrapper {
        mut solana_chain,
        chain_name: solana_chain_name,
        ..
    } = axelar_solana_setup().await;
    // Setup - EVM
    let (_evm_chain, evm_signer, its_contracts) = axelar_evm_setup().await;

    let test_its_canonical_token = evm_signer
        .deploy_axelar_test_canonical_token("Canonical Token".to_owned(), "CT".to_owned(), 18)
        .await
        .unwrap();

    let register_tx = its_contracts
        .interchain_token_factory
        .register_canonical_interchain_token(test_its_canonical_token.address())
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(register_tx.status.unwrap(), U64::one());

    let event_filter = its_contracts
        .interchain_token_service
        .interchain_token_id_claimed_filter();

    let token_id = event_filter
        .query()
        .await
        .unwrap()
        .first()
        .unwrap()
        .token_id;

    let deploy_tx = its_contracts
        .interchain_token_factory
        .deploy_remote_canonical_interchain_token(
            String::new(),
            test_its_canonical_token.address(),
            solana_chain_name.clone(),
            0_u128.into(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(deploy_tx.status.unwrap(), U64::one());

    let log: ContractCallFilter = its_contracts
        .gateway
        .contract_call_filter()
        .query()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("no logs found");

    let payload = log.payload.as_ref().to_vec();
    let payload_hash = solana_sdk::keccak::hash(&payload).to_bytes();
    let axelar_message = random_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

    // - The relayer relays the message to the Solana gateway
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![axelar_message.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    // - Relayer calls the Solana ITS program
    let instruction = axelar_solana_its::instructions::its_gmp_payload(
        &solana_chain.fixture.payer.pubkey(),
        &gateway_approved_command_pdas[0],
        &solana_chain.gateway_root_pda,
        axelar_message.into(),
        payload,
    )
    .expect("failed to create instruction");

    let tx1 = solana_chain
        .fixture
        .send_tx_with_metadata(&[instruction])
        .await;

    let log_msgs = tx1.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|tx_log| tx_log
            .as_str()
            .contains("Received DeployInterchainToken message")),
        "DeployInterchainToken message not received"
    );
    assert!(
        log_msgs
            .iter()
            .any(|tx_log| tx_log.as_str().contains(&hex::encode(token_id))),
        "Token Id doesn't match"
    );
}
