//! Error types

use num_derive::FromPrimitive;
use solana_program::program_error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
/// Errors of [Gas Service] program.
pub enum GasServiceError {
    /// InvalidInstruction
    #[error("invalid instruction")]
    InvalidInstruction,

    /// InvalidSystemAccount
    #[error("invalid system account")]
    InvalidSystemAccount,

    /// InvalidGasServiceRootPDAAccount
    #[error("invalid gas service root pda account")]
    InvalidGasServiceRootPDAAccount,

    /// SenderAccountIsNotWrittable
    #[error("sender account isn't writable")]
    SenderAccountIsNotWrittable,

    /// SenderAccountIsNotSigner
    #[error("sender account is not signer")]
    SenderAccountIsNotSigner,

    /// RootPDAAccountAlreadyInitialized
    #[error("root pda account already initialized")]
    RootPDAAccountAlreadyInitialized,

    /// ReceiverAccountIsNotWrittable
    #[error("receiver account isn't writable")]
    ReceiverAccountIsNotWrittable,

    /// SenderAccountIsNotExpectedAuthority
    #[error("unauthorized: sender account isn't the expected authority")]
    SenderAccountIsNotExpectedAuthority,

    /// InsufficientFundsForTransaction
    #[error("insufficient funds on the account to debit")]
    InsufficientFundsForTransaction,
}

impl From<GasServiceError> for ProgramError {
    fn from(e: GasServiceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
