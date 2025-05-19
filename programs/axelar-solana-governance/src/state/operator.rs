//! This module provides the tools to manage operator proposals.

use crate::seed_prefixes;
use crate::state::proposal::ExecutableProposal;
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Derives the operator proposal approved PDA for the given proposal hash.
#[must_use]
pub fn derive_managed_proposal_pda(proposal_hash: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::OPERATOR_MANAGED_PROPOSAL, proposal_hash],
        &crate::ID,
    )
}

/// Ensures the provided PDA matches the proposal hash and bump.
///
/// # Errors
///
/// Returns a [`ProgramError`] if the provided PDA does not match the derived
/// one.
pub fn ensure_correct_managed_proposal_pda(
    proposal_pda: &AccountInfo<'_>,
    proposal_managed_pda: &AccountInfo<'_>,
    proposal_hash: &[u8; 32],
) -> Result<u8, ProgramError> {
    let proposal = ExecutableProposal::load_from(&crate::id(), proposal_pda).map_err(|err| {
        msg!("Failed to load proposal for checking bumps: {:?}", err);
        ProgramError::InvalidArgument
    })?;

    ExecutableProposal::ensure_correct_proposal_pda(proposal_pda, proposal_hash, proposal.bump())?;

    let calculated_pda = Pubkey::create_program_address(
        &[
            seed_prefixes::OPERATOR_MANAGED_PROPOSAL,
            proposal_hash,
            &[proposal.managed_bump()],
        ],
        &crate::ID,
    )?;
    if calculated_pda != *proposal_managed_pda.key {
        msg!("Derived operator managed proposal PDA does not match provided one");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(proposal.managed_bump())
}
