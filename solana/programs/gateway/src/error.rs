//! Error types

use auth_weighted::error::AuthWeightedError;
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

    /// LowSignaturesWeight
    #[error("low signature weight")]
    LowSignaturesWeight,

    // 10
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

    // 15
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

    /// Invalid Gateway Config account
    #[error("Invalid Gateway Config account")]
    InvalidConfigAccount,

    // 20
    /// Invalid System Program account
    #[error("Invalid System Program account")]
    InvalidSystemAccount,

    /// Invalid Execute Data account
    #[error("Invalid Execute Data account")]
    InvalidExecuteDataAccount,

    /// Invalid Message ID account
    #[error("Invalid Message ID account")]
    InvalidMessageIDAccount,

    /// Failed to decode `execute_data`
    #[error("Falied to decode execute_data")]
    FailedToDecodeExecuteData,

    /// Arithmetic overflow
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,

    // 25
    /// Operator set epoch is different than the current epoch
    #[error("Operator set epoch is different than the current epoch.")]
    EpochMissmatch,

    /// Failed to decode a valid signature
    #[error("Failed to deserialize signature")]
    Secp256k1InvalidSignature,

    /// Failed to recover public key from message hash and recovery id
    #[error("Failed to recover public key from message hash and recovery id")]
    Secp256k1RecoveryFailed,

    /// All proof signers are invalid
    #[error("All proof signers are invalid")]
    AllSignersInvalid,

    /// Operator list was exhausted during proof validation
    #[error("Operator list was exhausted during proof validation")]
    OperatorsExhausted,
    // 30
}

impl From<GatewayError> for ProgramError {
    fn from(e: GatewayError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

/// TODO: Once we merge `auth-weighted` types into this crate, most of their
/// error variants should be removed as well.
impl From<AuthWeightedError> for GatewayError {
    fn from(error: AuthWeightedError) -> Self {
        use AuthWeightedError::*;
        match error {
            InvalidOperators => GatewayError::InvalidOperators,
            InvalidWeights => GatewayError::InvalidWeights,
            InvalidThreshold => GatewayError::InvalidThreshold,
            DuplicateOperators => GatewayError::DuplicateOperators,
            LowSignaturesWeight => GatewayError::LowSignaturesWeight,
            InvalidInstruction => GatewayError::InvalidInstruction,
            InvalidProgramID => GatewayError::InvalidProgramID,
            MalformedProof => GatewayError::MalformedProof,
            MalformedState => GatewayError::MalformedState,
            MalformedTransferOperatorshipParams => {
                GatewayError::MalformedTransferOperatorshipParams
            }
            EpochForHashNotFound => GatewayError::EpochForHashNotFound,
            EpochMissmatch => GatewayError::EpochMissmatch,
            Secp256k1RecoveryFailedInvalidSignature => {
                GatewayError::Secp256k1RecoveryFailedInvalidSignature
            }
            Secp256k1RecoveryFailedInvalidRecoveryId => {
                GatewayError::Secp256k1RecoveryFailedInvalidRecoveryId
            }
            Secp256k1RecoveryFailedInvalidHash => GatewayError::Secp256k1RecoveryFailedInvalidHash,
            ArithmeticOverflow => GatewayError::ArithmeticOverflow,
            Secp256k1RecoveryFailed => GatewayError::Secp256k1RecoveryFailed,
            Secp256k1InvalidSignature => GatewayError::Secp256k1InvalidSignature,
            AllSignersInvalid => GatewayError::AllSignersInvalid,
            OperatorsExhausted => GatewayError::OperatorsExhausted,
        }
    }
}
