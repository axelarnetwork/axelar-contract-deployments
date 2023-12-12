#![deny(missing_docs)]

//! Axelar Auth Weighted program for the Solana blockchain.

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod types;

// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("9J64GNeKqTzGQVAVW2MvdRf3Z1WiCnZqAxyhRAo2ouL9");

/// Checks that the supplied program ID is the correct one.
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
