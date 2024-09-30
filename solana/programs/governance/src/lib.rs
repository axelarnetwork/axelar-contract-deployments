//! # Multicall program
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;

solana_program::declare_id!("B3gam8xC15TDne4XtAVAvDDfqJFeSH6mv6sn6TanVJju");

/// Checks that the supplied program ID is the correct one
///
/// # Errors
///
/// If the program ID passed doesn't match the current program ID
#[inline]
pub fn check_program_account(program_id: Pubkey) -> ProgramResult {
    if program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

/// Seed prefixes for different PDAs initialized by the Governance program.
pub mod seed_prefixes {
    /// The main config for the governance
    pub const GOVERNANCE_CONFIG: &[u8; 10] = b"governance";
}
