//! # Multicall program
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;

solana_program::declare_id!("itswMJtRUe2vd46rb5kDmYzfBHHej4PyX4twgnbT1TG");

/// Seed prefixes for different PDAs initialized by the program
pub mod seed_prefixes {
    /// The seed prefix for deriving the ITS root PDA
    pub const ITS_SEED: &[u8] = b"interchain-token-service";

    /// The seed prefix for deriving the token manager PDA
    pub const TOKEN_MANAGER_SEED: &[u8] = b"token-manager";
}

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

/// Checks that the supplied program ID is the correct one
///
/// # Errors
///
/// If the bump cannot be used to derive the correct PDA
pub fn check_initialization_bump(
    bump: u8,
    its_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
) -> ProgramResult {
    let derived = Pubkey::create_program_address(
        &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref(), &[bump]],
        &crate::ID,
    )?;

    if derived != *its_pda {
        msg!("Derived PDA does not match expected PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Derives interchain token service root PDA
fn its_root_pda_internal(gateway_root_pda: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref()],
        program_id,
    )
}

/// Derives interchain token service root PDA
#[inline]
#[must_use]
pub fn its_root_pda(gateway_root_pda: &Pubkey) -> (Pubkey, u8) {
    its_root_pda_internal(gateway_root_pda, &crate::id())
}

fn token_manager_pda_internal(
    its_root_pda: &Pubkey,
    token_id: &[u8],
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.as_ref(),
            token_id,
        ],
        program_id,
    )
}

/// Derives the PDA for a [`TokenManager`].
#[inline]
#[must_use]
pub fn token_manager_pda(its_root_pda: &Pubkey, token_id: &[u8]) -> (Pubkey, u8) {
    token_manager_pda_internal(its_root_pda, token_id, &crate::id())
}
