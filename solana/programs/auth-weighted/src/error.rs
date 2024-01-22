//! Error types

use num_derive::FromPrimitive;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::secp256k1_recover::Secp256k1RecoverError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
/// Errors of [AuthWeighted] program.
pub enum AuthWeightedError {
    // 0
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

    /// LowSignaturesWeight
    #[error("low signature weight")]
    LowSignaturesWeight,
    // 5
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
    // 10
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
    // 15
    /// Failed to recover public key from message hash and recovery id
    #[error("Failed to recover public key from message hash and recovery id")]
    Secp256k1RecoveryFailed,

    /// Failed to decode a valid signature
    #[error("Failed to deserialize signature")]
    Secp256k1InvalidSignature,

    /// Arithmetic overflow
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,

    /// All proof signers are invalid
    #[error("All proof signers are invalid")]
    AllSignersInvalid,

    /// Operator list was exhausted during proof validation
    #[error("Operator list was exhausted during proof validation")]
    OperatorsExhausted,
    // 20
}

impl From<AuthWeightedError> for ProgramError {
    fn from(e: AuthWeightedError) -> Self {
        msg!("Error: {}", e);
        match e {
            AuthWeightedError::ArithmeticOverflow => ProgramError::ArithmeticOverflow,
            _ => ProgramError::Custom(e as u32),
        }
    }
}

impl From<Secp256k1RecoverError> for AuthWeightedError {
    fn from(solana_error: Secp256k1RecoverError) -> Self {
        use AuthWeightedError::*;
        use Secp256k1RecoverError::*;
        match solana_error {
            InvalidHash => Secp256k1RecoveryFailedInvalidHash,
            InvalidRecoveryId => Secp256k1RecoveryFailedInvalidRecoveryId,
            InvalidSignature => Secp256k1InvalidSignature,
        }
    }
}
