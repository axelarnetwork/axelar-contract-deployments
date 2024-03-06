use crate::sentinel::types::TransactionScannerMessage;
use solana_client::client_error::ClientError;
use solana_sdk::signature::{ParseSignatureError, Signature};
use thiserror::Error;
use tokio::{sync::mpsc::error::SendError, task::JoinError};

#[derive(Debug, Error)]
pub enum SentinelError {
    #[error("Solana RPC error - {0}")]
    SolanaRPCError(#[from] ClientError),
    #[error("Failed to parse byte vector into an UTF-8 string: {0}")]
    ByteVecParsing(std::string::FromUtf8Error),
    #[error("Database error - {0}")]
    Database(#[from] sqlx::Error),
    #[error("Failed to decode base58 string as a Solana signature: {0}")]
    SignatureParse(#[from] ParseSignatureError),
    #[error("Failed to send message to Axelar Verifier: {0}")]
    SendMessageError(String),
    #[error("Failed to send transaction signature for fetching its details: {0}")]
    SendSignatureError(#[from] SendError<Signature>),
    #[error("Failed to send proceesed transaction for event analysis: {0}")]
    SendTransactionError(#[from] SendError<TransactionScannerMessage>),
    #[error("Transaction Scanner stopped working unexpectedly without errors")]
    TransactionScannerStopped,
    #[error("Failed to await on a solana transaction fetch task")]
    FetchTransactionTaskJoinError(#[from] JoinError),
    #[error("Failed to decode solana transaction: {signature}")]
    TransactionDecode { signature: Signature },
    #[error("Solana Sentinel stopped working unexpectedly without errors")]
    Stopped,
    #[error(transparent)]
    NonFatal(#[from] NonFatalError),
}

/// Errors that shouldn't halt the Sentinel.
#[derive(Error, Debug)]
pub enum NonFatalError {
    #[error("Got wrong signature from RPC. Expected: {expected}, received: {received}")]
    WrongTransactionReceived {
        expected: Signature,
        received: Signature,
    },
    #[error("Got a transaction without meta attribute: {signature}")]
    TransactionWithoutMeta { signature: Signature },
    #[error("Got a transaction without logs: {signature}")]
    TransactionWithoutLogs { signature: Signature },
}
