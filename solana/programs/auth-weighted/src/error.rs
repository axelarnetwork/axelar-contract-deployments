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

    /// Operator set epoch is different than the current epoch
    #[error("Operator set epoch is different than the current epoch.")]
    EpochMissmatch,

    /// https://docs.rs/solana-program/latest/solana_program/secp256k1_recover/fn.secp256k1_recover.html#errors
    #[error("could not recover public key due to invalid signature")]
    Secp256k1RecoveryFailedInvalidSignature,

    /// Secp256k1RecoveryFailedInvalidRecoveryId
    #[error("could not recover public key due to invalid recovery id")]
    Secp256k1RecoveryFailedInvalidRecoveryId,

    /// Secp256k1RecoveryFailedInvalidHash
    #[error("could not recover public key due to invalid hash")]
    Secp256k1RecoveryFailedInvalidHash,

    /// Arithmetic overflow
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,
}

impl From<AuthWeightedError> for ProgramError {
    fn from(e: AuthWeightedError) -> Self {
        match e {
            AuthWeightedError::ArithmeticOverflow => ProgramError::ArithmeticOverflow,
            _ => ProgramError::Custom(e as u32),
        }
    }
}
