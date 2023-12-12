//! Error types

use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
/// Errors of [AuthWeighted] program.
pub enum AuthWeightedError {
    /// InvalidOperators
    #[error("invalid operators")]
    InvalidOperators,

    /// InvalidWeights
    #[error("invalid weights")]
    InvalidWeights,

    /// InvalidThreshold
    #[error("invalid treshold")]
    InvalidThreshold,

    /// DuplicateOperators
    #[error("duplicate operators")]
    DuplicateOperators,

    /// MalformedSigners
    #[error("malformed signers")]
    MalformedSigners,

    /// LowSignaturesWeight
    #[error("low signature weight")]
    LowSignaturesWeight,

    /// InvalidInstruction
    #[error("invalid instruction")]
    InvalidInstruction,

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

    /// DebugError
    #[error("use only for debugging")]
    DebugError,
}

impl From<AuthWeightedError> for ProgramError {
    fn from(e: AuthWeightedError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
