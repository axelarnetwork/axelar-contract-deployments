#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("VqMMNEMXqUagHieikoHz4YgFBusPs3kMFHN59yuwaoM");

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Derives the associated chain address and bump seed for the given wallet address
pub(crate) fn get_associated_chain_address_and_bump_seed_internal(
    wallet_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&wallet_address.to_bytes()], program_id)
}

/// Derives the associated chain address for the given wallet address
pub fn get_associated_chain_address(wallet_address: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[&wallet_address.to_bytes()], &crate::id()).0
}
