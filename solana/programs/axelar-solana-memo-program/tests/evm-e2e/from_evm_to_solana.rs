use std::str::FromStr;

use axelar_executable::axelar_message_primitives::command::DecodedCommand;
use axelar_executable::axelar_message_primitives::{DataPayload, DestinationProgramId};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_gateway::ContractCallFilter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::{axelar_gateway, axelar_memo};
use evm_contracts_test_suite::ContractMiddleware;
use itertools::Either;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::axelar_message::custom_message;

use crate::{axelar_evm_setup, axelar_solana_setup};

#[tokio::test]
async fn test_send_from_evm_to_solana() {
    // Setup - Solana
    let (mut solana_chain, gateway_root_pda, solana_operators) = axelar_solana_setup().await;
    // Setup - EVM
    let (_evm_chain, evm_signer, _evm_aw, evm_gateway, _operators) = axelar_evm_setup().await;
    let evm_memo = evm_signer
        .deploy_axelar_memo(evm_gateway.clone())
        .await
        .unwrap();

    // Test-scoped Constants
    let solana_id = "solana-localnet";
    let memo = "🐪🐪🐪🐪";
    let random_account_used_by_ix = solana_sdk::signature::Keypair::new();

    // Action:
    // - send message from EVM memo program to EVM gateway
    let log = call_evm_gateway(
        &evm_memo,
        solana_id,
        memo,
        &random_account_used_by_ix,
        &evm_gateway,
    )
    .await;
    // - Solana operators approve the message
    // - The relayer relays the message to the Solana gateway
    let (decoded_payload, msg_from_evm_axelar) = prase_evm_log_into_axelar_message(&log);
    let messages = vec![Either::Left(msg_from_evm_axelar.clone())];
    let (gateway_approved_command_pdas, gatewa_execute_data, _) = solana_chain
        .fully_approve_messages(&gateway_root_pda, &messages, &solana_operators)
        .await;
    // - Relayer calls the Solana memo program with the memo payload coming from the
    //   EVM memo program
    let tx = call_execute_on_solana_memo_program(
        &gatewa_execute_data,
        &decoded_payload,
        &gateway_approved_command_pdas,
        gateway_root_pda,
        &mut solana_chain,
    )
    .await;

    // Assert
    // We can get the memo from the logs
    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains(&format!(
            "{:?}-{}-{}",
            random_account_used_by_ix.pubkey(),
            false,
            false
        ))),
        "expected memo not found in logs"
    );
}

async fn call_execute_on_solana_memo_program(
    gatewa_execute_data: &gateway::state::GatewayExecuteData,
    decoded_payload: &DataPayload<'_>,
    gateway_approved_command_pdas: &[solana_sdk::pubkey::Pubkey],
    gateway_root_pda: solana_sdk::pubkey::Pubkey,
    solana_chain: &mut test_fixtures::test_setup::TestFixture,
) -> solana_program_test::BanksTransactionResultWithMetadata {
    let DecodedCommand::ApproveContractCall(approved_message) =
        gatewa_execute_data.command_batch.commands[0].clone()
    else {
        panic!("expected ApproveContractCall command")
    };
    let ix = axelar_executable::construct_axelar_executable_ix(
        approved_message,
        decoded_payload.encode().unwrap(),
        gateway_approved_command_pdas[0],
        gateway_root_pda,
    )
    .unwrap();
    let recent_blockhash = solana_chain
        .banks_client
        .get_latest_blockhash()
        .await
        .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&solana_chain.payer.pubkey()),
        &[&solana_chain.payer],
        recent_blockhash,
    );
    let tx = solana_chain
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();
    assert!(tx.result.is_ok(), "transaction failed");
    tx
}

fn prase_evm_log_into_axelar_message(
    log: &ContractCallFilter,
) -> (
    DataPayload<'_>,
    test_fixtures::test_setup::connection_router::Message,
) {
    let decoded_payload = DataPayload::decode(log.payload.as_ref()).unwrap();
    let msg_from_evm_axelar = custom_message(
        DestinationProgramId(
            solana_sdk::pubkey::Pubkey::from_str(log.destination_contract_address.as_str())
                .unwrap(),
        ),
        decoded_payload.clone(),
    )
    .unwrap();
    (decoded_payload, msg_from_evm_axelar)
}

async fn call_evm_gateway(
    evm_memo: &axelar_memo::AxelarMemo<ContractMiddleware>,
    solana_id: &str,
    memo: &str,
    random_account_used_by_ix: &solana_sdk::signature::Keypair,
    evm_gateway: &axelar_gateway::AxelarGateway<ContractMiddleware>,
) -> ContractCallFilter {
    let _receipt = evm_memo
        .send_to_solana(
            axelar_solana_memo_program::id().to_string(),
            solana_id.as_bytes().to_vec().into(),
            memo.as_bytes().to_vec().into(),
            vec![
                evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo::SolanaAccountRepr {
                    pubkey: random_account_used_by_ix.pubkey().to_bytes(),
                    is_signer: false,
                    is_writable: false,
                },
            ],
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    let logs: Vec<ContractCallFilter> = evm_gateway
        .contract_call_filter()
        .from_block(0u64)
        .query()
        .await
        .unwrap();

    logs.into_iter().next().expect("no logs found")
}
