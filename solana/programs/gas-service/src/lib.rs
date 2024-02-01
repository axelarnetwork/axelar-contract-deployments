//! Axelar Gas Service program for the Solana blockchain.
#![deny(missing_docs)]
#[allow(clippy::too_many_arguments)]
pub mod accounts;
mod entrypoint;
pub mod error;
pub mod events;
pub mod instruction;
pub mod processor;

pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

// The ID here is fake, and used only for development.
// TODO: replace with real ID
solana_program::declare_id!("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL");

/// Checks that the supplied program ID is the correct one.
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Finds the program root PDA.
pub fn get_gas_service_root_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&[]], &crate::id())
}
