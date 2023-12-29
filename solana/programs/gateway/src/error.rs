//! Error types

use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GatewayError {
    // 0
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Invalid message payload hash
    #[error("Invalid message payload hash")]
    InvalidMessagePayloadHash,

    /// Byte serialization error
    #[error("Byte serialization error")]
    ByteSerializationError,

    /// Incorrect root state account
    #[error("Incorrect root state account")]
    IncorrectAccountAddr,

    /// Account already initialized
    #[error("Account already initialized")]
    AccountAlreadyInitialized,

    // 5
    /// Invalid operators
    #[error("Invalid operators")]
    InvalidOperators,

    /// Invalid weights
    #[error("Invalid weights")]
    InvalidWeights,

    /// Invalid threshold
    #[error("Invalid threshold")]
    InvalidThreshold,

    /// DuplicateOperators
    #[error("duplicate operators")]
    DuplicateOperators,

    /// MalformedSigners
    #[error("malformed signers")]
    MalformedSigners,
    // 10
    /// LowSignaturesWeight
    #[error("low signature weight")]
    LowSignaturesWeight,

    /// InvalidProgramID
    #[error("invalid program id")]
    InvalidProgramID,

    /// MalformedProof
    #[error("malformed proof body")]
    MalformedProof,

    /// MalformedState
    #[error("malformed state body")]
    MalformedState,

    /// MalformedTransferOperatorshipParams
    #[error("malformed transfer operatorship body")]
    MalformedTransferOperatorshipParams,

    // 15
    /// EpochForHashNotFound
    #[error("could not find requested key")]
    EpochForHashNotFound,

    /// https://docs.rs/solana-program/latest/solana_program/secp256k1_recover/fn.secp256k1_recover.html#errors
    #[error("could not recover public key due to invalid signature")]
    Secp256k1RecoveryFailedInvalidSignature,

    /// Secp256k1RecoveryFailedInvalidRecoveryId
    #[error("could not recover public key due to invalid recovery id")]
    Secp256k1RecoveryFailedInvalidRecoveryId,

    /// Secp256k1RecoveryFailedInvalidHash
    #[error("could not recover public key due to invalid hash")]
    Secp256k1RecoveryFailedInvalidHash,

    /// Invalid Account Address
    #[error("Invalid Account Address")]
    InvalidAccountAddress,

    // 20
    /// Invalid Gateway Config account
    #[error("Invalid Gateway Config account")]
    InvalidConfigAccount,

    /// Invalid System Program account
    #[error("Invalid System Program account")]
    InvalidSystemAccount,

    /// Invalid Execute Data account
    #[error("Invalid Execute Data account")]
    InvalidExecuteDataAccount,

    /// Invalid Message ID account
    #[error("Invalid Message ID account")]
    InvalidMessageIDAccount,
}

impl From<GatewayError> for ProgramError {
    fn from(e: GatewayError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
