use axelar_solana_memo_program_old::instruction::call_gateway_with_memo;
use ethers_core::abi::AbiEncode;
use ethers_core::utils::keccak256;
use gateway::events::{CallContract, GatewayEvent};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::program_test;

#[tokio::test]
async fn test_successfully_send_to_gateway() {
    // Setup
    let mut solana_chain = program_test().await;
    let memo = "🐪🐪🐪🐪";
    let destination_address = ethers_core::types::Address::random().encode_hex();
    let destination_chain = "ethereum".to_string();

    // Action: send message to gateway
    let transaction = Transaction::new_signed_with_payer(
        &[call_gateway_with_memo(
            &solana_chain.gateway_root_pda,
            &solana_chain.fixture.payer.pubkey(),
            memo.to_string(),
            destination_chain.clone(),
            destination_address.clone(),
        )
        .unwrap()],
        Some(&solana_chain.fixture.payer.pubkey()),
        &[&solana_chain.fixture.payer],
        solana_chain
            .fixture
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap(),
    );
    let tx = solana_chain
        .fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert!(tx.result.is_ok(), "transaction failed");

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
            sender: solana_chain.fixture.payer.pubkey().to_bytes(),
            destination_chain,
            destination_address,
            payload: memo.as_bytes().to_vec(),
            payload_hash: keccak256(memo.as_bytes())
        }),
        "Mismatched gateway event"
    );
}
