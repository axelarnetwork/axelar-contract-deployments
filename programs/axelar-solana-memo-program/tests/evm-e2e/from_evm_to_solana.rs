use crate::{axelar_evm_setup, axelar_solana_setup, MemoProgramWrapper};
use anyhow::{bail, Context, Result};
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
use solana_sdk::transaction::TransactionError;
use std::fmt;
use thiserror::Error;

#[derive(Copy, Clone)]
struct MemoTestCase {
    symbol: char,
    count: usize,
}

impl MemoTestCase {
    fn is_large(&self) -> bool {
        // magic number, but it should be something that can possibly be logged by the Solana Gateway
        self.size_in_bytes() > 30
    }

    fn size_in_bytes(&self) -> usize {
        self.symbol.len_utf8() * self.count
    }

    fn memo(&self) -> String {
        std::iter::repeat(self.symbol).take(self.count).collect()
    }

    fn find_memo_in_log(&self, log: &str) -> bool {
        if self.is_large() {
            log.contains(self.symbol) & log.contains(&self.count.to_string())
        } else {
            log.contains(&self.memo())
        }
    }
}

impl fmt::Display for MemoTestCase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memo test case {} ({} chars, {} bytes)",
            self.symbol,
            self.count,
            self.size_in_bytes()
        )
    }
}

/// This is a benchmark disguised as a test because Solana's SBF toolchain only supports
/// `cargo test-sbf` and not `cargo bench-sbf`. It uses binary search to find the maximum
/// number of emoji symbols that can be successfully sent from EVM to Solana in a single
/// transaction.
///
/// Note: This is not a proper test as it's meant for performance measurement rather than
/// correctness verification. It's marked as a test to work within Solana's SBF tooling
/// limitations.
///
/// To use the results: Run this "test" and note the maximum successful case value,
/// then use that value as a reference for actual implementation limits.
#[ignore]
#[tokio::test]
async fn bench_send_from_evm_to_solana() -> Result<()> {
    // Binary search for the maximum limit
    let symbol = '🐇';
    let mut low = 1;
    let mut high = 20_000; // Start with an upper bound beyond we know will fail
    let mut max_successful = None;

    while low <= high {
        let mid = low + (high - low) / 2;
        let test_case = MemoTestCase { symbol, count: mid };

        println!(
            "Trying count: {} (size: {} bytes)",
            mid,
            test_case.size_in_bytes()
        );

        match test_send_from_evm_to_solana_single_case(test_case).await {
            Ok(()) => {
                println!("✅ Success: {test_case}");
                max_successful = Some(test_case);
                low = mid + 1;
            }
            Err(error) => {
                println!("❌ Failed: {test_case}. Error: {error}");
                high = mid - 1;
            }
        }
    }

    bail!(
        "Maximum successful: {}",
        max_successful.expect("No test cases succeeded"),
    );
}

#[ignore]
#[tokio::test]
async fn test_send_from_evm_to_solana_all_cases() -> Result<()> {
    // Each emoji is 4 bytes
    let base_case = MemoTestCase {
        symbol: '🐪',
        count: 4, // 16 bytes
    };
    let max_case = MemoTestCase {
        symbol: '🐇',
        count: 2488, // 9,952 bytes, our current maximum
    };

    for test_case in [base_case, max_case] {
        println!("Running {test_case}");
        test_send_from_evm_to_solana_single_case(test_case).await?;
    }

    // Confidence check: MemeTestCase fails with one extra symbol above our current maximum.
    let beyond_max_case = MemoTestCase {
        symbol: '🐐',
        count: max_case.count + 1,
    };
    let should_fail = test_send_from_evm_to_solana_single_case(beyond_max_case).await;
    let test_error = should_fail.expect_err("should've failed");

    assert!(matches!(
        test_error.downcast_ref::<MemoTestError>(),
        Some(MemoTestError::OutOfMemory)
    ));

    Ok(())
}

/// Error type for memo testing operations.
///
/// Since we need to run the same test with different inputs, we switched from
/// assert-based panics to proper error handling. This way we can check each test
/// case individually and continue testing even when one fails.
#[derive(Error, Debug)]
pub enum MemoTestError {
    #[error("Failed to deploy Axelar memo contract")]
    DeploymentError,

    #[error("Failed to call EVM gateway")]
    EvmGatewayError,

    #[error("Failed to create a signing session and approve messages")]
    SigningError,

    #[error("No merkleised message available after signing")]
    NoMerkleisedMessage,

    #[error("Failed to execute transaction: {0}")]
    TransactionError(TransactionError),

    #[error("Memory allocation failed, out of memory")]
    OutOfMemory,

    #[error("Transaction metadata not available")]
    NoTransactionMetadata,

    #[error("Expected memo not found in logs for test case: {0}")]
    MemoNotFound(String),

    #[error("Failed to deserialize counter data")]
    CounterDeserializationError,

    #[error("Counter value mismatch: expected {expected}, got {actual}")]
    CounterMismatch { expected: u64, actual: u64 },

    #[error("Failed to decode Axelar message payload")]
    PayloadDecodingError,

    #[error("Failed to encode Axelar message payload")]
    PayloadEncodingError,

    #[error("Failed to compute payload hash")]
    PayloadHashError,

    #[error("Failed to send transaction to Solana")]
    SolanaTxSendError,

    #[error("Failed to query EVM gateway logs")]
    EvmLogsQueryError,

    #[error("No contract call logs found")]
    NoContractCallLogs,

    #[error("Failed to await transaction receipt")]
    TxReceiptError,
}

async fn test_send_from_evm_to_solana_single_case(test_case: MemoTestCase) -> Result<()> {
    // Setup - Solana
    let MemoProgramWrapper {
        mut solana_chain,
        counter_pda,
    } = axelar_solana_setup().await;

    // Setup - EVM
    let (_evm_chain, evm_signer, evm_gateway, _weighted_signers, _domain_separator) =
        axelar_evm_setup().await;

    // Deploy Axelar memo contract
    let evm_memo = evm_signer
        .deploy_axelar_memo(evm_gateway.clone(), None)
        .await
        .context(MemoTestError::DeploymentError)?;

    // Test-scoped Constants
    let solana_id = "solana-localnet";
    let memo = test_case.memo();

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
        &memo,
        vec![counter_account],
        &evm_gateway,
    )
    .await
    .context(MemoTestError::EvmGatewayError)?;

    // - Solana signers approve the message
    // - The relayer relays the message to the Solana gateway

    let (decoded_payload, msg_from_evm_axelar) = prase_evm_log_into_axelar_message(&log)?;

    let merkelised_message = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[msg_from_evm_axelar])
        .await
        .map_err(|_| MemoTestError::SigningError)?
        .into_iter()
        .next()
        .ok_or(MemoTestError::NoMerkleisedMessage)?;

    let tx = solana_chain
        .execute_on_axelar_executable(
            merkelised_message.leaf.message,
            &decoded_payload
                .encode()
                .context(MemoTestError::PayloadEncodingError)?,
        )
        .await
        .or_else(|error| {
            let Some(tx_error) = error.result.err() else {
                unreachable!()
            };
            if error
                .metadata
                .ok_or(MemoTestError::NoTransactionMetadata)?
                .log_messages
                .iter()
                .any(|log| log.contains("memory allocation failed, out of memory"))
            {
                Err(MemoTestError::OutOfMemory)
            } else {
                Err(MemoTestError::TransactionError(tx_error))
            }
        })?;

    // Assert
    let log_msgs = tx
        .metadata
        .ok_or(MemoTestError::NoTransactionMetadata)?
        .log_messages;

    if !log_msgs.iter().any(|log| test_case.find_memo_in_log(log)) {
        bail!(MemoTestError::MemoNotFound(test_case.to_string()))
    }

    let counter = solana_chain
        .get_account(&counter_pda, &axelar_solana_memo_program::ID)
        .await;

    let counter = Counter::try_from_slice(&counter.data)
        .context(MemoTestError::CounterDeserializationError)?;

    if counter.counter != 1 {
        bail!(MemoTestError::CounterMismatch {
            expected: 1,
            actual: counter.counter,
        });
    }

    Ok(())
}

fn prase_evm_log_into_axelar_message(
    log: &ContractCallFilter,
) -> Result<(AxelarMessagePayload<'_>, Message)> {
    let decoded_payload = AxelarMessagePayload::decode(log.payload.as_ref())
        .context(MemoTestError::PayloadDecodingError)?;

    let payload_hash = decoded_payload
        .hash()
        .context(MemoTestError::PayloadHashError)?;

    let message = Message {
        cc_id: CrossChainId {
            chain: "ethereum".to_string(),
            id: "transaction-id-321".to_string(),
        },
        source_address: log.sender.encode_hex_with_prefix(),
        destination_chain: log.destination_chain.clone(),
        destination_address: log.destination_contract_address.clone(),
        payload_hash: *payload_hash.0,
    };

    Ok((decoded_payload, message))
}

async fn call_evm_gateway(
    evm_memo: &axelar_memo::AxelarMemo<ContractMiddleware>,
    solana_id: &str,
    memo: &str,
    solana_accounts_to_provide: Vec<SolanaAccountRepr>,
    evm_gateway: &axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
) -> Result<ContractCallFilter> {
    // Send transaction and wait for receipt
    let _receipt = evm_memo
        .send_to_solana(
            axelar_solana_memo_program::id().to_string(),
            solana_id.as_bytes().to_vec().into(),
            memo.as_bytes().to_vec().into(),
            solana_accounts_to_provide,
        )
        .send()
        .await
        .context(MemoTestError::SolanaTxSendError)?
        .await
        .context(MemoTestError::TxReceiptError)?
        .context(MemoTestError::TxReceiptError)?;

    // Query logs
    let logs: Vec<ContractCallFilter> = evm_gateway
        .contract_call_filter()
        .from_block(0u64)
        .query()
        .await
        .context(MemoTestError::EvmLogsQueryError)?;

    // Get the first log or return error
    logs.into_iter()
        .next()
        .ok_or(MemoTestError::NoContractCallLogs.into())
}
