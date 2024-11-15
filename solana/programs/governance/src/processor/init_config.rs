//! Initialize Governance Config Account with the provided Governance Config
//! data.

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::ensure_valid_governance_root_pda;
use crate::seed_prefixes;
use crate::state::GovernanceConfig;

/// Initializes the Governance Config Account with the provided Governance
/// Config.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    governance_config: GovernanceConfig,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    let bump = governance_config.bump;

    // Check: Config account uses the canonical bump.
    ensure_valid_governance_root_pda(bump, root_pda.key)?;

    // Check: PDA Account is not initialized
    root_pda.check_uninitialized_pda()?;

    program_utils::init_rkyv_pda::<{ GovernanceConfig::LEN }, _>(
        payer,
        root_pda,
        program_id,
        system_account,
        governance_config,
        &[seed_prefixes::GOVERNANCE_CONFIG, &[bump]],
    )?;

    Ok(())
}
