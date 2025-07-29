//! # Governance program

#![allow(clippy::unneeded_field_pattern)]

use program_utils::ensure_single_feature;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

pub mod entrypoint;
pub mod events;
pub mod instructions;
pub mod processor;
pub mod sol_types;
pub mod state;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("govmXi41LqLpRpKUd79wvAh9MmpoMzXk7gG4Sqmucx9");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("govXsQZx7cZcMBWQWkk4gq8eoA4MKkYi3G1sCzLPcqa");

#[cfg(feature = "testnet")]
solana_program::declare_id!("goveBmEV286hz1LLGRurkSXD5fgRYEmiVFMXK4Vp6zk");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("gov1111111111111111111111111111111111111111");

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
    pub const GOVERNANCE_CONFIG: &[u8] = b"governance";
    /// The seed that determines a proposal PDA
    pub const PROPOSAL_PDA: &[u8] = b"proposal";
    /// The seed that derives a PDA which holds a status that
    /// signals an operator can operate a proposal (like executing it
    /// regardless of the ETA).
    pub const OPERATOR_MANAGED_PROPOSAL: &[u8] = b"operator-managed-proposal";
}
