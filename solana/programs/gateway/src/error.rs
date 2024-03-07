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
    /// The presented operator set was empty.
    #[error("Operator array cannot be empty")]
    EmptyOperators,

    /// Used for attempts to update the GatewayConfig with an already existing
    /// operator set.
    // TODO: use a more specific error name.
    #[error("duplicate operators")]
    DuplicateOperators,

    /// LowSignaturesWeight
    #[error("low signature weight")]
    LowSignaturesWeight,

    /// InvalidProgramID
    #[error("invalid program id")]
    InvalidProgramID,

    /// MalformedProof
    #[error("malformed proof body")]
    MalformedProof,
    // 10
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
    // 15
    /// Secp256k1RecoveryFailedInvalidHash
    #[error("could not recover public key due to invalid hash")]
    Secp256k1RecoveryFailedInvalidHash,

    /// Invalid Account Address
    #[error("Invalid Account Address")]
    InvalidAccountAddress,

    /// Invalid Gateway Config account
    #[error("Invalid Gateway Config account")]
    InvalidConfigAccount,

    /// Invalid System Program account
    #[error("Invalid System Program account")]
    InvalidSystemAccount,

    /// Invalid Execute Data account
    #[error("Invalid Execute Data account")]
    InvalidExecuteDataAccount,
    // 20
    /// Invalid Approved Message account
    #[error("Invalid Approved Message account")]
    InvalidApprovedMessageAccount,

    /// Failed to decode `execute_data`
    #[error("Falied to decode execute_data")]
    FailedToDecodeExecuteData,

    /// Arithmetic overflow
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,

    /// Operator set epoch is different than the current epoch
    #[error("Operator set epoch is different than the current epoch.")]
    EpochMissmatch,

    /// Failed to decode a valid signature
    #[error("Failed to deserialize signature")]
    Secp256k1InvalidSignature,
    // 25
    /// Failed to recover public key from message hash and recovery id
    #[error("Failed to recover public key from message hash and recovery id")]
    Secp256k1RecoveryFailed,

    /// All proof signers are invalid
    #[error("All proof signers are invalid")]
    AllSignersInvalid,

    /// Operator list was exhausted during proof validation
    #[error("Operator list was exhausted during proof validation")]
    OperatorsExhausted,

    /// The sum of operator weights was smaller than the required threshold.
    #[error("Insufficient operator weight to resolve operatorship transfer")]
    InsufficientOperatorWeight,

    /// Proposed operator array was either unordered or contained duplicate
    /// entries.
    #[error("Operators array must be sorted (asc) and unique")]
    UnorderedOrDuplicateOperators,

    // 30
    /// Thresold was presented as zero, which is an invalid value.
    #[error("Threshold cannot be equal to zero")]
    ZeroThreshold,

    /// Used if the operator set for an incoming proof has a sufficiently oldn
    /// epoch.
    #[error("Operators' epoch is outdated")]
    OutdatedOperatorsEpoch,

    /// Operators' epoch was set to zero, which is an invalid value.
    #[error("Operators' epoch cannot be equal to zero")]
    EpochZero,

    /// The provided caller is not authorized to execute this instruction
    #[error("The provided caller is not authorized to execute this instruction")]
    InvalidExecutor,

    /// The GatewayApprovedMessage PDA has not been approved yet! Wait for the
    /// `execute` instruction to be called.
    #[error("The GatewayApprovedMessage PDA has not been approved yet! Wait for the `execute` instruction to be called.")]
    GatewayMessageNotApproved,

    // 35
    /// The `caller` is not a signer, which is required verify that he wants to
    /// execute the message.
    #[error("The `caller` is not a signer, which is required verify that he wants to execute the message.")]
    MismatchedAllowedCallers,
}

impl From<GatewayError> for ProgramError {
    fn from(e: GatewayError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
