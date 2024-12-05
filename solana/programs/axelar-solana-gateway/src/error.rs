//! Error types

use num_derive::{FromPrimitive, ToPrimitive};
use solana_program::program_error::ProgramError;

/// Errors that may be returned by the Token program.
#[repr(u32)]
#[derive(Clone, Debug, Eq, thiserror::Error, FromPrimitive, ToPrimitive, PartialEq)]
pub enum GatewayError {
    /// Error indicating an underflow occurred during epoch calculation.
    #[error("Epoch calculation resulted in an underflow")]
    EpochCalculationOverflow = 0,

    /// Error indicating the provided signers are invalid.
    #[error("Verifier Set too old")]
    VerifierSetTooOld = 1,

    /// Invalid Weight threshold
    #[error("Invalid Weight threshold")]
    InvalidWeightThreshold = 2,

    /// Data LEN mismatch when trying to read bytemucked data
    #[error("Invalid Bytemucked data len")]
    BytemuckDataLenInvalid = 3,
}

impl From<GatewayError> for ProgramError {
    fn from(e: GatewayError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
