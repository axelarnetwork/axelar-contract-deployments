use axelar_solana_gateway::processor::{
    CallContractEvent, CallContractOffchainDataEvent, GatewayEvent,
};
use axelar_solana_gateway_test_fixtures::gateway::{get_gateway_events, ProgramInvocationState};
use axelar_solana_memo_program::get_counter_pda;
use axelar_solana_memo_program::instruction::{
    call_gateway_with_memo, call_gateway_with_offchain_memo,
};
use ethers_core::abi::AbiEncode;
use pretty_assertions::assert_eq;
use solana_program_test::tokio;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::signer::Signer;

use crate::program_test;

#[tokio::test]
async fn test_successfully_send_to_gateway() {
    // Setup
    let mut solana_chain = program_test().await;
    let memo = "🐪🐪🐪🐪";
    let destination_address = ethers_core::types::Address::random().encode_hex();
    let destination_chain = "ethereum".to_string();
    let (counter_pda, counter_bump) = get_counter_pda(&solana_chain.gateway_root_pda);
    let initialize = axelar_solana_memo_program::instruction::initialize(
        &solana_chain.fixture.payer.pubkey().clone(),
        &solana_chain.gateway_root_pda.clone(),
        &(counter_pda, counter_bump),
    )
    .unwrap();
    solana_chain.send_tx(&[initialize]).await.unwrap();

    // Action: send message to gateway
    let call_gateway_with_memo = call_gateway_with_memo(
        &solana_chain.gateway_root_pda,
        &counter_pda,
        memo.to_string(),
        destination_chain.clone(),
        destination_address.clone(),
        &axelar_solana_gateway::ID,
    )
    .unwrap();
    let tx = solana_chain
        .send_tx(&[call_gateway_with_memo])
        .await
        .unwrap();

    // Assert
    // We can get the memo from the logs
    let emitted_events = get_gateway_events(&tx).pop().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GatewayEvent::CallContract(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    assert_eq!(
        emitted_event,
        &CallContractEvent {
            sender_key: axelar_solana_memo_program::ID,
            destination_chain,
            destination_contract_address: destination_address,
            payload: memo.as_bytes().to_vec(),
            payload_hash: solana_sdk::keccak::hash(memo.as_bytes()).0
        },
        "Mismatched gateway event"
    );
}

#[tokio::test]
async fn test_successfully_send_to_gateway_with_offchain_data() {
    // Setup
    let mut solana_chain = program_test().await;
    let memo = "🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪
    🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪🐪"
        .to_string()
        .replace(['\n', ' '], "");
    let destination_address = ethers_core::types::Address::random().encode_hex();
    let destination_chain = "ethereum".to_string();
    let (counter_pda, counter_bump) = get_counter_pda(&solana_chain.gateway_root_pda);
    let initialize = axelar_solana_memo_program::instruction::initialize(
        &solana_chain.fixture.payer.pubkey().clone(),
        &solana_chain.gateway_root_pda.clone(),
        &(counter_pda, counter_bump),
    )
    .unwrap();
    solana_chain.send_tx(&[initialize]).await.unwrap();

    // Action: send message to gateway
    let call_gateway_with_memo = call_gateway_with_offchain_memo(
        &solana_chain.gateway_root_pda,
        &counter_pda,
        memo.clone(),
        destination_chain.clone(),
        destination_address.clone(),
        &axelar_solana_gateway::ID,
    )
    .unwrap();
    let tx = solana_chain
        .send_tx(&[
            ComputeBudgetInstruction::set_compute_unit_limit(250_000_000),
            call_gateway_with_memo,
        ])
        .await
        .unwrap();

    // Assert
    // We can get the memo from the logs
    let emitted_events = get_gateway_events(&tx).pop().unwrap();
    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };
    let [(_, GatewayEvent::CallContractOffchainData(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };
    assert_eq!(
        emitted_event,
        &CallContractOffchainDataEvent {
            sender_key: axelar_solana_memo_program::ID,
            destination_chain,
            destination_contract_address: destination_address,
            payload_hash: solana_sdk::keccak::hash(memo.as_bytes()).0
        },
        "Mismatched gateway event"
    );
}
