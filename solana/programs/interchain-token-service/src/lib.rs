#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod instruction;
pub mod processor;
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("4ENH4KjzfcQwyXYr6SJdaF2nhMoGqdZJ2Hk5MoY9mU2G");

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
