use axelar_executable::AxelarMessagePayload;
use axelar_solana_encoding::types::messages::{CrossChainId, Message};
use axelar_solana_memo_program::state::Counter;
use borsh::BorshDeserialize;
use ethers_core::utils::hex::ToHexExt;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::ContractCallFilter;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_memo::SolanaAccountRepr;
use evm_contracts_test_suite::evm_contracts_rs::contracts::{
    axelar_amplifier_gateway, axelar_memo,
};
use evm_contracts_test_suite::ContractMiddleware;
use solana_program_test::tokio;

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
        .deploy_axelar_memo(evm_gateway.clone())
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
    let merkelised_message = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[msg_from_evm_axelar])
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    // - Relayer calls the Solana memo program with the memo payload coming from the
    //   EVM memo program
    let tx = solana_chain
        .execute_on_axelar_executable(
            merkelised_message.leaf.message,
            &decoded_payload.encode().unwrap(),
        )
        .await
        .unwrap();

    // Assert
    // We can get the memo from the logs
    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("🐪🐪🐪🐪")),
        "expected memo not found in logs"
    );

    let counter = solana_chain
        .get_account(&counter_pda, &axelar_solana_memo_program::ID)
        .await;
    let counter = Counter::try_from_slice(&counter.data).unwrap();
    assert_eq!(counter.counter, 1);
}

fn prase_evm_log_into_axelar_message(
    log: &ContractCallFilter,
) -> (AxelarMessagePayload<'_>, Message) {
    let decoded_payload = AxelarMessagePayload::decode(log.payload.as_ref()).unwrap();
    let message = Message {
        cc_id: CrossChainId {
            chain: "ethereum".to_string(),
            id: "transaction-id-321".to_string(),
        },
        source_address: log.sender.encode_hex_with_prefix(),
        destination_chain: log.destination_chain.clone(),
        destination_address: log.destination_contract_address.clone(),
        payload_hash: *decoded_payload.hash().unwrap().0,
    };
    (decoded_payload, message)
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
            axelar_solana_memo_program::id().to_string(),
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
