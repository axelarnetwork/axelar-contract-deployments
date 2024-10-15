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

    /// The seed prefix for deriving the interchain token PDA
    pub const INTERCHAIN_TOKEN_SEED: &[u8] = b"interchain-token";
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

pub(crate) fn its_root_pda_internal(
    gateway_root_pda: &Pubkey,
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref(), &[bump]],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref()],
                &crate::id(),
            )
        })
}

/// Tries to create the ITS root PDA using the provided bump, falling back to
/// `find_program_address` if the bump is `None` or invalid.
#[inline]
#[must_use]
pub fn its_root_pda(gateway_root_pda: &Pubkey, bump: u8) -> (Pubkey, u8) {
    its_root_pda_internal(gateway_root_pda, Some(bump))
}

/// Derives interchain token service root PDA
#[inline]
#[must_use]
pub fn find_its_root_pda(gateway_root_pda: &Pubkey) -> (Pubkey, u8) {
    its_root_pda_internal(gateway_root_pda, None)
}

pub(crate) fn token_manager_pda_internal(
    interchain_token_pda: &Pubkey,
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::TOKEN_MANAGER_SEED,
                    interchain_token_pda.as_ref(),
                    &[bump],
                ],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::TOKEN_MANAGER_SEED,
                    interchain_token_pda.as_ref(),
                ],
                &crate::id(),
            )
        })
}
/// Tries to create the PDA for a [`Tokenmanager`] using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[inline]
#[must_use]
pub fn token_manager_pda(interchain_token_pda: &Pubkey, bump: u8) -> (Pubkey, u8) {
    token_manager_pda_internal(interchain_token_pda, Some(bump))
}

/// Derives the PDA for a [`TokenManager`].
#[inline]
#[must_use]
pub fn find_token_manager_pda(interchain_token_pda: &Pubkey) -> (Pubkey, u8) {
    token_manager_pda_internal(interchain_token_pda, None)
}

pub(crate) fn interchain_token_pda_internal(
    its_root_pda: &Pubkey,
    token_id: &[u8],
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::INTERCHAIN_TOKEN_SEED,
                    its_root_pda.as_ref(),
                    token_id,
                    &[bump],
                ],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::INTERCHAIN_TOKEN_SEED,
                    its_root_pda.as_ref(),
                    token_id,
                ],
                &crate::id(),
            )
        })
}

/// Tries to create the PDA for an `InterchainToken` using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[inline]
#[must_use]
pub fn interchain_token_pda(its_root_pda: &Pubkey, token_id: &[u8], bump: u8) -> (Pubkey, u8) {
    interchain_token_pda_internal(its_root_pda, token_id, Some(bump))
}

/// Derives the PDA for an interchain token account.
#[inline]
#[must_use]
pub fn find_interchain_token_pda(its_root_pda: &Pubkey, token_id: &[u8]) -> (Pubkey, u8) {
    interchain_token_pda_internal(its_root_pda, token_id, None)
}
