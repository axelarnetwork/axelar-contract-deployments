use solana_sdk::{hash::ParseHashError, transaction::TransactionError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration Error: {0}")]
    ConfigError(String),

    #[error("I/O Error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization Error (JSON): {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Serialization Error (Hex): {0}")]
    HexError(#[from] hex::FromHexError),

    #[error("Serialization Error (Base58): {0}")]
    Base58Error(#[from] bs58::decode::Error),

    #[error("Serialization Error (Bincode): {0}")]
    BincodeError(#[from] bincode::error::EncodeError),

    #[error("Packaging Error: {0}")]
    PackagingError(String),

    #[error("Signing Error: {0}")]
    SigningError(String),

    #[error("Hardware Wallet Error: {0}")]
    HardwareWalletError(String),

    #[error("Key Not Found: {0}")]
    KeyNotFoundError(String),

    #[error("Signature Combination Error: {0}")]
    CombinationError(String),

    #[error("Broadcasting Error: {0}")]
    BroadcastError(String),

    #[error("Sending error: {0}")]
    SendError(String),

    #[error("Invalid Input: {0}")]
    InvalidInput(String),

    #[error("Blockchain Interaction Error: {0}")]
    ChainError(String),

    #[error("Invalid Network Type: {0}")]
    InvalidNetworkType(String),

    #[error("Solana RPC Client Error: {0}")]
    SolanaClientError(#[from] solana_client::client_error::ClientError),

    #[error("Solana Program/SDK Error: {0}")]
    SolanaProgSdkError(String),

    #[error("Inconsistent State: {0}")]
    InconsistentState(String),

    #[error("Feature Not Implemented: {0}")]
    NotImplemented(String),

    #[error("Failed to parse hash: {0}")]
    ParseHashError(#[from] ParseHashError),

    #[error("Transaction error: {0}")]
    TransactionError(#[from] TransactionError),

    #[error("Unknown Error: {0}")]
    Unknown(String),
}

impl From<solana_sdk::pubkey::ParsePubkeyError> for AppError {
    fn from(err: solana_sdk::pubkey::ParsePubkeyError) -> Self {
        AppError::InvalidInput(format!("Invalid Solana Pubkey: {}", err))
    }
}
impl From<solana_sdk::signature::ParseSignatureError> for AppError {
    fn from(err: solana_sdk::signature::ParseSignatureError) -> Self {
        AppError::InvalidInput(format!("Invalid Solana Signature: {}", err))
    }
}

impl From<solana_sdk::program_error::ProgramError> for AppError {
    fn from(err: solana_sdk::program_error::ProgramError) -> Self {
        AppError::SolanaProgSdkError(format!("Solana Program Error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
