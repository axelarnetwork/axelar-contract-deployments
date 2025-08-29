//! Initialize Governance Config Account with the provided Governance Config
//! data.

use program_utils::{account_array_structs, pda::ValidPDA};
use role_management::processor::ensure_upgrade_authority;
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use crate::seed_prefixes;
use crate::state::{validate_config, GovernanceConfig};

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    GovernanceConfigInfo,
    // Struct whose attributes are of type `AccountMeta`
    GovernanceConfigMeta,
    // Attributes
    payer,
    program_data,
    root_pda,
    system_account
}

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
    let GovernanceConfigInfo {
        payer,
        program_data,
        root_pda,
        system_account,
    } = GovernanceConfigInfo::from_account_iter(&mut accounts.iter())?;

    ensure_upgrade_authority(program_id, payer, program_data)?;

    validate_config(&governance_config)?;

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

    program_utils::pda::init_pda::<GovernanceConfig>(
        payer,
        root_pda,
        program_id,
        system_account,
        governance_config,
        &[seed_prefixes::GOVERNANCE_CONFIG, &[bump]],
    )?;

    Ok(())
}
