#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::hash::hash;
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

/// Derives the associated chain address and bump seed for the given wallet
/// address
pub(crate) fn get_associated_chain_address_and_bump_seed_internal(
    wallet_address: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&wallet_address.as_ref()], program_id)
}

/// Derives the associated chain address for the given wallet address
pub fn get_associated_chain_address(wallet_address: &Pubkey) -> Pubkey {
    get_associated_chain_address_and_bump_seed_internal(wallet_address, &crate::id()).0
}

/// Derives the associated trusted address and bump seed for the given chain
/// address and address
pub(crate) fn get_associated_trusted_address_account_and_bump_seed_internal(
    associated_chain_address: &Pubkey,
    chain_name: &str,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let chain_name = hash(chain_name.as_bytes());
    Pubkey::find_program_address(
        &[&chain_name.to_bytes(), &associated_chain_address.as_ref()],
        program_id,
    )
}

/// Derives the associated trusted address for the given chain address and
/// address
pub fn get_associated_trusted_address(
    associated_chain_address: &Pubkey,
    chain_name: &str,
) -> Pubkey {
    get_associated_trusted_address_account_and_bump_seed_internal(
        associated_chain_address,
        chain_name,
        &crate::id(),
    )
    .0
}
