use std::borrow::Cow;

use axelar_solana_memo_program::instruction::call_gateway_with_memo;
use ethers_core::utils::keccak256;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo::ReceivedMemoFilter;
use gateway::events::{CallContract, GatewayEvent};
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

use crate::{axelar_evm_setup, axelar_solana_setup};

#[tokio::test]
async fn test_send_from_solana_to_evm() {
    // Setup - Solana
    let (mut solana_chain, gateway_root_pda, _signers, _counter) = axelar_solana_setup().await;
    // Setup - EVM
    let (evm_chain, evm_signer, _evm_aw, evm_gateway, mut operators) = axelar_evm_setup().await;
    let evm_memo = evm_signer
        .deploy_axelar_memo(evm_gateway.clone())
        .await
        .unwrap();

    // Test-scoped Constants
    let solana_id = "solana-localnet";
    let memo = "🐪🐪🐪🐪";
    let destination_address: ethers_core::types::Address = evm_memo.address();
    let destination_chain = "ethereum".to_string().into_bytes();

    // Action:
    // - send message from Solana memo program to Solana gateway
    let call_contract = call_solana_gateway(
        &gateway_root_pda,
        &mut solana_chain,
        memo,
        destination_chain,
        &destination_address,
    )
    .await;
    // - EVM operators sign the contract call
    let (command_id, signed_weighted_execute_input) =
        evm_prepare_approve_contract_call(solana_id, &call_contract, &evm_chain, &mut operators);
    // - The relayer relays the contract call to the EVM gateway
    let _tx_reciept = evm_gateway
        .execute(signed_weighted_execute_input.into())
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    // Assert - we check that the contract call was approved
    let is_approved = evm_gateway
        .is_contract_call_approved(
            command_id,
            solana_id.to_string(),
            call_contract.sender.to_string(),
            ethers_core::types::Address::from_slice(call_contract.destination_address.as_slice()),
            call_contract.payload_hash,
        )
        .await
        .unwrap();
    assert!(is_approved, "contract call was not approved");
    assert_eq!(
        keccak256(call_contract.payload.clone()),
        call_contract.payload_hash
    );
    assert_eq!(
        evm_memo.address(),
        ethers_core::types::Address::from_slice(call_contract.destination_address.as_slice())
    );

    // Action - Relayer calls the EVM memo program with the payload
    evm_memo
        .execute(
            command_id,
            solana_id.to_string(),
            call_contract.sender.to_string(),
            call_contract.payload.into(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    // Assert - event was emitted on EVM
    let logs: Vec<ReceivedMemoFilter> = evm_memo
        .received_memo_filter()
        .from_block(0u64)
        .query()
        .await
        .unwrap();
    let log = logs.into_iter().next().expect("no logs found");
    assert_eq!(log.memo_message, memo, "memo does non match");
    // Assert - message counter was updated
    assert_eq!(
        evm_memo.messages_received().await.unwrap(),
        ethers_core::types::U256::from(1),
        "message counter needs to be updated"
    );
}

fn evm_prepare_approve_contract_call(
    solana_id: &str,
    call_contract: &CallContract,
    evm_chain: &evm_contracts_test_suite::chain::TestBlockchain,
    operators: &mut evm_contracts_test_suite::evm_operators::OperatorSet,
) -> ([u8; 32], Vec<u8>) {
    let approve_contract_call_command =
        evm_contracts_test_suite::evm_operators::get_approve_contract_call(
            solana_id.to_string(),
            call_contract.sender.to_string(),
            ethers_core::types::Address::from_slice(call_contract.destination_address.as_slice()),
            call_contract.payload_hash,
            [11; 32], // random values - not checked by anything
            42.into(),
        );
    // build command batch
    let command_id = [42; 32]; // random uniqu command id
    let command_batch = evm_contracts_test_suite::evm_operators::build_command_batch(
        evm_chain.anvil.chain_id(),
        &[command_id],
        vec!["approveContractCall".to_string()],
        vec![approve_contract_call_command],
    );
    // get signed weighted execute input
    let signed_weighted_execute_input =
        evm_contracts_test_suite::evm_operators::get_signed_weighted_execute_input(
            command_batch,
            operators,
        );
    (command_id, signed_weighted_execute_input)
}

async fn call_solana_gateway(
    gateway_root_pda: &solana_sdk::pubkey::Pubkey,
    solana_fixture: &mut test_fixtures::test_setup::TestFixture,
    memo: &str,
    destination_chain: Vec<u8>,
    destination_address: &ethers_core::types::H160,
) -> CallContract {
    let transaction = Transaction::new_signed_with_payer(
        &[call_gateway_with_memo(
            gateway_root_pda,
            &solana_fixture.payer.pubkey(),
            memo.to_string(),
            destination_chain,
            destination_address.as_bytes().to_vec(),
        )
        .unwrap()],
        Some(&solana_fixture.payer.pubkey()),
        &[&solana_fixture.payer],
        solana_fixture
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap(),
    );
    let tx = solana_fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert!(tx.result.is_ok(), "transaction failed");

    let log_msgs = tx.metadata.unwrap().log_messages;
    let gateway_event = log_msgs
        .iter()
        .find_map(GatewayEvent::parse_log)
        .expect("Gateway event was not emitted?");
    let GatewayEvent::CallContract(Cow::Owned(call_contract)) = gateway_event else {
        panic!("Expected CallContract event, got {:?}", gateway_event);
    };

    call_contract
}
