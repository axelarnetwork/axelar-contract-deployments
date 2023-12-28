#![deny(missing_docs)]

//! Axelar Gateway program for the Solana blockchain

pub mod accounts;
mod entrypoint;
pub mod error;
pub mod events;
pub mod instructions;
pub mod processor;
pub mod types;

// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

solana_program::declare_id!("4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a");

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: Pubkey) -> ProgramResult {
    if program_id != id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the account is initialized.
pub fn check_initialized(v: u64) -> ProgramResult {
    if v != 0 {
        return Err(GatewayError::IncorrectAccountAddr.into());
    }
    Ok(())
}

/// Finds the program root PDA.
pub fn find_root_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&[]], &crate::id())
}
