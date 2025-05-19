use axelar_executable::AxelarMessagePayload;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use axelar_solana_memo_program::instruction::AxelarMemoInstruction;
use axelar_solana_memo_program::state::Counter;
use borsh::BorshDeserialize as _;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::{
    AxelarAmplifierGateway, ContractCallFilter,
};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_solana_multicall::{
    AxelarSolanaCall, AxelarSolanaMultiCall, SolanaAccountRepr, SolanaGatewayPayload,
};
use evm_contracts_test_suite::ContractMiddleware;
use solana_program_test::tokio;

use crate::{axelar_evm_setup, axelar_solana_setup, TestContext};

#[tokio::test]
async fn test_send_from_evm_to_solana() {
    // Setup - Solana
    let TestContext {
        mut solana_chain,
        memo_program_counter_pda,
    } = axelar_solana_setup().await;
    // Setup - EVM
    let (_evm_chain, evm_signer, evm_gateway, _weighted_signers, _domain_separator) =
        axelar_evm_setup().await;

    let evm_multicall = evm_signer
        .deploy_solana_multicall(evm_gateway.clone())
        .await
        .unwrap();

    // Test-scoped Constants
    let solana_id = "solana-localnet";

    let mut calls = Vec::new();
    let counter_account = SolanaAccountRepr {
        pubkey: memo_program_counter_pda.to_bytes(),
        is_signer: false,
        is_writable: true,
    };
    for memo in &["Call A", "Call B", "Call C"] {
        calls.push(AxelarSolanaCall {
            destination_program: axelar_solana_memo_program::id().to_bytes(),
            payload: SolanaGatewayPayload {
                execute_payload: borsh::to_vec(&AxelarMemoInstruction::ProcessMemo {
                    memo: (*memo).to_string(),
                })
                .expect("failed to create multicall instruction")
                .into(),
                accounts: vec![counter_account.clone()],
            },
        });
    }

    let evm_gateway_event_log =
        call_evm_gateway(&evm_multicall, solana_id, calls, &evm_gateway).await;
    let decoded_payload =
        AxelarMessagePayload::decode(evm_gateway_event_log.payload.as_ref()).unwrap();
    let mut message = random_message();
    message.destination_address = axelar_solana_multicall::id().to_string();
    message.payload_hash = *decoded_payload.hash().unwrap();

    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let tx = solana_chain
        .execute_on_axelar_executable(
            merkelised_message.leaf.message,
            &decoded_payload.encode().unwrap(),
        )
        .await
        .unwrap();

    let log_msgs = tx.metadata.unwrap().log_messages;
    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("Call A")),
        "expected memo not found in logs"
    );

    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("Call B")),
        "expected memo not found in logs"
    );

    assert!(
        log_msgs.iter().any(|log| log.as_str().contains("Call C")),
        "expected memo not found in logs"
    );

    let counter = solana_chain
        .get_account(&memo_program_counter_pda, &axelar_solana_memo_program::ID)
        .await;
    let counter = Counter::try_from_slice(&counter.data).unwrap();
    assert_eq!(counter.counter, 3);
}

async fn call_evm_gateway(
    evm_multicall: &AxelarSolanaMultiCall<ContractMiddleware>,
    solana_id: &str,
    calls: Vec<AxelarSolanaCall>,
    evm_gateway: &AxelarAmplifierGateway<ContractMiddleware>,
) -> ContractCallFilter {
    let _result = evm_multicall
        .multi_call(
            calls,
            solana_id.as_bytes().to_vec().into(),
            axelar_solana_multicall::id().to_string(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    let logs: Vec<ContractCallFilter> = evm_gateway
        .contract_call_filter()
        .from_block(0_u64)
        .query()
        .await
        .unwrap();

    logs.into_iter().next().expect("no logs found")
}
