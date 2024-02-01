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

solana_program::declare_id!("4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a");

/// Checks that the supplied program ID is the correct one
#[inline]
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Get the root PDA and bump seed for the given program ID.
#[inline]
pub fn get_gateway_root_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[], &ID)
}
