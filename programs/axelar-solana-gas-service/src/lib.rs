//! Axelar Gas Service program for the Solana blockchain
#![allow(clippy::little_endian_bytes)]
pub mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version.
use program_utils::ensure_single_feature;
pub use solana_program;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("gasd4em72NAm7faq5dvjN5GkXE59dUkTThWmYDX95bK");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("gaspfz1SLfPr1zmackMVMgShjkuCGPZ5taN8wAfwreW");

#[cfg(feature = "testnet")]
solana_program::declare_id!("gaspFGXoWNNMMaYGhJoNRMNAp8R3srFeBmKAoeLgSYy");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("gas1111111111111111111111111111111111111111");

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
/// Given a `program_id`, a `salt` (32-byte array), and an `operator` (`Pubkey`), this function
/// uses [`Pubkey::find_program_address`] to return the derived PDA and its associated bump seed.
#[inline]
#[must_use]
pub fn get_config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seed_prefixes::CONFIG_SEED], &crate::ID)
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
pub fn assert_valid_config_pda(bump: u8, expected_pubkey: &Pubkey) -> Result<(), ProgramError> {
    let derived_pubkey =
        Pubkey::create_program_address(&[seed_prefixes::CONFIG_SEED, &[bump]], &crate::ID)
            .expect("invalid bump for the config pda");

    if &derived_pubkey == expected_pubkey {
        Ok(())
    } else {
        msg!("Error: Invalid Config PDA");
        Err(ProgramError::IncorrectProgramId)
    }
}
