//! This module provides the tools to manage operator proposals.

use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::seed_prefixes;

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
    pda: &Pubkey,
    proposal_hash: &[u8; 32],
    bump: u8,
) -> Result<(), ProgramError> {
    let calculated_pda = Pubkey::create_program_address(
        &[
            seed_prefixes::OPERATOR_MANAGED_PROPOSAL,
            proposal_hash,
            &[bump],
        ],
        &crate::ID,
    )?;
    if calculated_pda != *pda {
        msg!("Derived operator managed proposal PDA does not match provided one");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use solana_sdk::instruction::AccountMeta;

    use super::*;
    use crate::instructions::builder::IxBuilder;

    #[test]
    fn test_ensure_correct_proposal_operator_pda() {
        let hash = [
            231, 53, 251, 125, 82, 133, 64, 207, 92, 6, 137, 25, 193, 97, 18, 24, 90, 204, 45, 161,
            101, 30, 86, 79, 64, 48, 200, 139, 11, 34, 94, 232,
        ];
        let (pda, bump) = derive_managed_proposal_pda(&hash);
        assert!(ensure_correct_managed_proposal_pda(&pda, &hash, bump).is_ok());
    }

    #[test]
    fn test_builder_pda_checks_alignment() {
        let ix_builder = IxBuilder::new().with_proposal_data(
            Pubkey::new_unique(),
            0,
            1234,
            None,
            &[AccountMeta::new(Pubkey::new_unique(), true)],
            vec![0],
        );

        let pda = ix_builder.proposal_operator_marker_pda();
        let bump: u8 = ix_builder
            .proposal_call_data()
            .proposal_operator_managed_bump()
            .unwrap();
        let hash = ix_builder.proposal_hash();
        assert!(ensure_correct_managed_proposal_pda(&pda, &hash, bump).is_ok());
    }
}
