//! Axelar Gas Service program for the Solana blockchain
#![allow(clippy::little_endian_bytes)]
pub mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("gasFkyvr4LjK3WwnMGbao3Wzr67F88TmhKmi4ZCXF9K");

/// Seed prefixes for PDAs created by this program
pub mod seed_prefixes {
    /// The seed used when deriving the configuration PDA.
    pub const CONFIG_SEED: &[u8] = b"gas-service";
}

/// Checks that the provided `program_id` matches the current program’s ID.
///
/// # Errors
///
/// - if the provided `program_id` does not match.
#[inline]
pub fn check_program_account(program_id: Pubkey) -> Result<(), ProgramError> {
    if program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Derives the configuration PDA for this program.
///
/// Given a `program_id`, a `salt` (32-byte array), and an `authority` (`Pubkey`), this function
/// uses [`Pubkey::find_program_address`] to return the derived PDA and its associated bump seed.
#[inline]
#[must_use]
pub fn get_config_pda(program_id: &Pubkey, salt: &[u8; 32], authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::CONFIG_SEED, salt, authority.as_ref()],
        program_id,
    )
}

/// Checks that the given `expected_pubkey` matches the derived PDA for the provided parameters.
///
/// # Panics
/// - if the seeds + bump don't result in a valid PDA
///
/// # Errors
///
/// - if the derived PDA does not match the `expected_pubkey`.
#[inline]
#[track_caller]
pub fn assert_valid_config_pda(
    bump: u8,
    salt: &[u8; 32],
    authority: &Pubkey,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = Pubkey::create_program_address(
        &[
            seed_prefixes::CONFIG_SEED,
            salt,
            authority.as_ref(),
            &[bump],
        ],
        &crate::ID,
    )
    .expect("invalid bump for the config pda");

    if &derived_pubkey == expected_pubkey {
        Ok(())
    } else {
        msg!("Error: Invalid Config PDA ");
        Err(ProgramError::IncorrectProgramId)
    }
}
