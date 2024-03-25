use std::fmt::Display;

use solana_sdk::signature::Signature;
use tokio::task::JoinHandle;

use super::{
    error::TransactionScannerError,
    transaction_scanner::transaction_retriever::TransactionRetrieverError,
};

pub struct SolanaTransaction {
    pub signature: Signature,
    pub logs: Vec<String>,
    pub block_time: Option<i64>,
    pub slot: u64,
}

impl Display for SolanaTransaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Delegate formatting to the `signature` field.`
        self.signature.fmt(f)
    }
}

pub enum TransactionScannerMessage {
    /// Error that terminated the transaction scanner task.
    Terminated(TransactionScannerError),
    /// Typical message with the produced work.
    /// Contains the handle to a task that resolves into a [`SolanaTransaction`].
    Message(JoinHandle<Result<SolanaTransaction, TransactionRetrieverError>>),
}
