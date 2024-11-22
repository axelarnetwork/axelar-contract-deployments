use axelar_solana_gateway::events::{CallContract, GatewayEvent};
use axelar_solana_memo_program::get_counter_pda;
use axelar_solana_memo_program::instruction::call_gateway_with_memo;
use ethers_core::abi::AbiEncode;
use pretty_assertions::assert_eq;
use solana_program_test::tokio;
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
    let log_msgs = dbg!(tx.metadata.unwrap().log_messages);

    let gateway_event = log_msgs
        .iter()
        .find_map(GatewayEvent::parse_log)
        .expect("Gateway event was not emitted?");
    let gateway_event = gateway_event.parse();
    assert_eq!(
        gateway_event,
        &GatewayEvent::CallContract(CallContract {
            sender: counter_pda.to_bytes(),
            destination_chain,
            destination_address,
            payload: memo.as_bytes().to_vec(),
            payload_hash: solana_sdk::keccak::hash(memo.as_bytes()).0
        }),
        "Mismatched gateway event"
    );
}
