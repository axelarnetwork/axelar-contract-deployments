use solana_sdk::signature::Signature;
use tokio::task::JoinHandle;

use super::error::SentinelError;

pub struct SolanaTransaction {
    pub signature: Signature,
    pub logs: Vec<String>,
    pub block_time: Option<i64>,
    pub slot: u64,
}

pub enum TransactionScannerMessage {
    /// Error that terminated the transaction scanner task.
    Terminated(Box<SentinelError>),
    /// Typical message with the produced work.
    /// Contains the handle to a task that resolves into a [`SolanaTransaction`].
    Message(JoinHandle<Result<SolanaTransaction, Box<SentinelError>>>),
}
