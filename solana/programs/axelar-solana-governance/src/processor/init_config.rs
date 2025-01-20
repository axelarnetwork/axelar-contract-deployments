//! Initialize Governance Config Account with the provided Governance Config
//! data.

use program_utils::ValidPDA;
use role_management::processor::ensure_upgrade_authority;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

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
    mut governance_config: GovernanceConfig,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let program_data = next_account_info(accounts_iter)?;
    let root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    ensure_upgrade_authority(program_id, payer, program_data)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }

    let (address, bump) = GovernanceConfig::pda();

    if address != *root_pda.key {
        msg!("Derived PDA does not match provided PDA");
        return Err(ProgramError::InvalidArgument);
    }

    governance_config.bump = bump;

    // Check: PDA Account is not initialized
    root_pda.check_uninitialized_pda()?;

    program_utils::init_pda::<GovernanceConfig>(
        payer,
        root_pda,
        program_id,
        system_account,
        governance_config,
        &[seed_prefixes::GOVERNANCE_CONFIG, &[bump]],
    )?;

    Ok(())
}
