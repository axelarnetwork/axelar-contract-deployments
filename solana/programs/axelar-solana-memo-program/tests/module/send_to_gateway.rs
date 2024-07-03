use std::borrow::Cow;

use axelar_solana_memo_program::instruction::call_gateway_with_memo;
use ethers_core::utils::keccak256;
use gateway::events::GatewayEvent;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::test_setup::TestFixture;
use test_fixtures::test_signer::create_signer_with_weight;

use crate::program_test;

#[tokio::test]
async fn test_succesfully_send_to_gateway() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128),
        create_signer_with_weight(4_u128),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers),
            Pubkey::new_unique(),
        )
        .await;
    let memo = "🐪🐪🐪🐪";
    let destination_address = ethers_core::types::Address::random().0.to_vec();
    let destination_chain = "ethereum".to_string().into_bytes();

    // Action: send message to gateway
    let transaction = Transaction::new_signed_with_payer(
        &[call_gateway_with_memo(
            &gateway_root_pda,
            &fixture.payer.pubkey(),
            memo.to_string(),
            destination_chain.clone(),
            destination_address.clone(),
        )
        .unwrap()],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    let tx = fixture
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
    assert_eq!(
        gateway_event,
        GatewayEvent::CallContract(Cow::Owned(gateway::events::CallContract {
            sender: fixture.payer.pubkey(),
            destination_chain,
            destination_address: destination_address.to_vec(),
            payload_hash: keccak256(memo.as_bytes()),
            payload: memo.as_bytes().to_vec(),
        })),
        "Mismatched gateway event"
    );
}
