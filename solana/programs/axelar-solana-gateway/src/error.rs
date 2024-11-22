//! Error types

use num_derive::{FromPrimitive, ToPrimitive};
use solana_program::program_error::ProgramError;
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, ToPrimitive, PartialEq)]
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
    /// The presented signer set was empty.
    #[error("Signer set cannot be empty")]
    EmptySignerSet,

    /// Used for attempts to update the GatewayConfig with an already existing
    /// signer set.
    // TODO: use a more specific error name.
    #[error("duplicate signer set")]
    DuplicateSignerSet,

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

    /// MalformedTransferSignerSetParams
    #[error("malformed rotate signers body")]
    MalformedRotateSignerSetParams,

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
    #[error("Failed to decode execute_data")]
    FailedToDecodeExecuteData,

    /// Arithmetic overflow
    #[error("Program arithmetic overflowed")]
    ArithmeticOverflow,

    /// Signer Set set epoch is different than the current epoch
    #[error("Signer Set set epoch is different than the current epoch.")]
    EpochMismatch,

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

    /// Signer Set list was exhausted during proof validation
    #[error("Signer Set list was exhausted during proof validation")]
    SignerSetExhausted,

    /// The sum of signer set weights was smaller than the required threshold.
    #[error("Insufficient signer weight to rotate signers set")]
    InsufficientSignerWeight,

    /// Proposed signer sets was either unordered or contained duplicate
    /// entries.
    #[error("Signer set array must be sorted (asc) and unique")]
    UnorderedOrDuplicateSignerSet,

    // 30
    /// Threshold was presented as zero, which is an invalid value.
    #[error("Threshold cannot be equal to zero")]
    ZeroThreshold,

    /// Used if the signer set for an incoming proof has a sufficiently old
    /// epoch.
    #[error("Signer set epoch is outdated")]
    OutdatedSignerSetEpoch,

    /// Signer set epoch was set to zero, which is an invalid value.
    #[error("Signer set epoch cannot be equal to zero")]
    EpochZero,

    /// The provided caller is not authorized to execute this instruction
    #[error("The provided caller is not authorized to execute this instruction")]
    InvalidExecutor,

    /// The GatewayApprovedMessage PDA has not been approved yet! Wait for the
    /// `execute` instruction to be called.
    #[error("The GatewayApprovedCommand PDA has not been approved yet! Wait for the `execute` instruction to be called.")]
    GatewayCommandNotApproved,

    // 35
    /// The `caller` is not a signer, which is required verify that he wants to
    /// execute the message.
    #[error("The `caller` is not a signer, which is required verify that he wants to execute the message.")]
    MismatchedAllowedCallers,

    /// Auth weighted error
    #[error("Auth weighted error")]
    AxelarAuthWeightedError,

    /// The GatewayCommandStatusMessage is not pending
    #[error(
        "The instruction expected the GatewayCommandStatusMessage to be pending, but it was not."
    )]
    GatewayCommandStatusNotPending,

    /// Failed to parse string as a valid public key
    #[error("Failed to parse string as a valid public key")]
    PublicKeyParseError,
}

impl From<GatewayError> for ProgramError {
    fn from(e: GatewayError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
