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

solana_program::declare_id!("4ENH4KjzfcQwyXYr6SJdaF2nhMoGqdZJ2Hk5MoY9mU2G");

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Derives interchain token service root PDA
pub(crate) fn get_interchain_token_service_root_pda_internal(
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&gateway_root_pda.as_ref(), &gas_service_root_pda.as_ref()],
        program_id,
    )
}

/// Derives interchain token service root PDA
pub fn get_interchain_token_service_root_pda(
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Pubkey {
    get_interchain_token_service_root_pda_internal(
        gateway_root_pda,
        gas_service_root_pda,
        &crate::id(),
    )
    .0
}
