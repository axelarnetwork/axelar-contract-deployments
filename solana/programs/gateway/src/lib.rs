#![deny(missing_docs)]

//! Axelar Gateway program for the Solana blockchain

mod entrypoint;
mod error;
pub mod events;
pub mod instruction;
pub mod processor;
// Export current sdk types for downstream users building with a different sdk
// version
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("VqMMNEMXqUagHieikoHz4YgFBusPs3kMFHN59yuwaoM");

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: Pubkey) -> ProgramResult {
    if program_id != id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
