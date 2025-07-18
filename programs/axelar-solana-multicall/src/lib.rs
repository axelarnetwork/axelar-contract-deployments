//! # Multicall program
use axelar_solana_gateway::ensure_single_feature;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

mod entrypoint;
pub mod instructions;
pub mod processor;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("mce2hozrGNRHP5qxScDvYyZ1TzhiH8tLLKxwo8DDNQT");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("mcHYeFvgcAsQqQDesRjbNQ7viuJgyn726pCWti4YgAi");

#[cfg(feature = "testnet")]
solana_program::declare_id!("mcjS7gsuNvNYD5AcrAaeMtS3hUPGDaJTekXSDuweAgJ");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("mc11111111111111111111111111111111111111111");

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
