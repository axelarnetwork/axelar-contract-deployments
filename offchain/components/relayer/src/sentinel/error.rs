use solana_client::client_error::ClientError;
use thiserror::Error;
use tokio::task::JoinError;

use super::transaction_scanner::signature_scanner::SignatureScannerError;
use super::transaction_scanner::transaction_retriever::TransactionRetrieverError;
use super::transaction_scanner::InternalError;

#[derive(Debug, Error)]
pub enum SentinelError {
    #[error("Solana RPC error - {0}")]
    SolanaRPCError(#[from] ClientError),
    #[error("Failed to parse byte vector into an UTF-8 string: {0}")]
    ByteVecParsing(std::string::FromUtf8Error),
    #[error("Database error - {0}")]
    Database(#[from] sqlx::Error),
    #[error("Solana Sentinel stopped working unexpectedly without errors")]
    Stopped,
    #[error(transparent)]
    TransactionScanner(#[from] TransactionScannerError),
    #[error("transaction scanner channel was closed")]
    TransactionScannerChannelClosed,
    #[error("Failed to await on a solana transaction fetch task")]
    FetchTransactionTaskJoinError(#[from] JoinError),
    #[error("Failed to send message to Axelar Verifier: {0}")]
    SendMessageError(String),
}

#[derive(Error, Debug)]
pub enum TransactionScannerError {
    #[error(transparent)]
    SignatureScanner(#[from] SignatureScannerError),
    #[error(transparent)]
    TransactionRetriever(#[from] TransactionRetrieverError),
    #[error(transparent)]
    Internal(#[from] InternalError),
}

impl From<SignatureScannerError> for SentinelError {
    fn from(error: SignatureScannerError) -> Self {
        let transaction_scanner: TransactionScannerError = error.into();
        SentinelError::TransactionScanner(transaction_scanner)
    }
}

impl From<TransactionRetrieverError> for SentinelError {
    fn from(error: TransactionRetrieverError) -> Self {
        let transaction_scanner: TransactionScannerError = error.into();
        SentinelError::TransactionScanner(transaction_scanner)
    }
}
