use std::str::FromStr;

use axelar_executable_old::axelar_message_primitives::DataPayload;
use axelar_rkyv_encoding::types::Message;
use axelar_solana_memo_program_old::state::Counter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo::SolanaAccountRepr;
use evm_contracts_test_suite::evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_memo,
};
use evm_contracts_test_suite::ContractMiddleware;
use gateway::commands::OwnedCommand;
use solana_program_test::tokio;
use test_fixtures::axelar_message::custom_message;

use crate::{axelar_evm_setup, axelar_solana_setup, MemoProgramWrapper};

#[tokio::test]
async fn test_send_from_evm_to_solana() {
    // Setup - Solana
    let MemoProgramWrapper {
        mut solana_chain,
        counter_pda,
    } = axelar_solana_setup().await;
    // Setup - EVM
    let (_evm_chain, evm_signer, evm_gateway, _weighted_signers, _domain_separator) =
        axelar_evm_setup().await;
    let evm_memo = evm_signer
        .deploy_axelar_memo(evm_gateway.clone(), None)
        .await
        .unwrap();

    // Test-scoped Constants
    let solana_id = "solana-localnet";
    let memo = "🐪🐪🐪🐪";

    // Action:
    // - send message from EVM memo program to EVM gateway
    let counter_account = SolanaAccountRepr {
        pubkey: counter_pda.to_bytes(),
        is_signer: false,
        is_writable: true,
    };
    let log = call_evm_gateway(
        &evm_memo,
        solana_id,
        memo,
        vec![counter_account],
        &evm_gateway,
    )
    .await;
    // - Solana signers approve the message
    // - The relayer relays the message to the Solana gateway
    let (decoded_payload, msg_from_evm_axelar) = prase_evm_log_into_axelar_message(&log);
    let (gateway_approved_command_pdas, _, _) = solana_chain
        .fixture
        .fully_approve_messages(
            &solana_chain.gateway_root_pda,
            vec![msg_from_evm_axelar.clone()],
            &solana_chain.signers,
            &solana_chain.domain_separator,
        )
        .await;

    let approve_message_command = OwnedCommand::ApproveMessage(msg_from_evm_axelar);
    // - Relayer calls the Solana memo program with the memo payload coming from the
    //   EVM memo program
    let tx = solana_chain
        .fixture
        .call_execute_on_axelar_executable(
            &approve_message_command,
            &decoded_payload,
            &gateway_approved_command_pdas[0],
            &solana_chain.gateway_root_pda,
        )
        .await;

    // Assert
    // We can get the memo from the logs
    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );
    let counter = solana_chain
        .fixture
        .get_account::<Counter>(&counter_pda, &axelar_solana_memo_program_old::ID)
        .await;
    assert_eq!(counter.counter, 1);
}

fn prase_evm_log_into_axelar_message(log: &ContractCallFilter) -> (DataPayload<'_>, Message) {
    let decoded_payload = DataPayload::decode(log.payload.as_ref()).unwrap();
    let msg_from_evm_axelar = custom_message(
        solana_sdk::pubkey::Pubkey::from_str(log.destination_contract_address.as_str()).unwrap(),
        &decoded_payload,
    );
    (decoded_payload, msg_from_evm_axelar)
}

async fn call_evm_gateway(
    evm_memo: &axelar_memo::AxelarMemo<ContractMiddleware>,
    solana_id: &str,
    memo: &str,
    solana_accounts_to_provide: Vec<SolanaAccountRepr>,
    evm_gateway: &axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
) -> ContractCallFilter {
    let _receipt = evm_memo
        .send_to_solana(
            axelar_solana_memo_program_old::id().to_string(),
            solana_id.as_bytes().to_vec().into(),
            memo.as_bytes().to_vec().into(),
            solana_accounts_to_provide,
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
