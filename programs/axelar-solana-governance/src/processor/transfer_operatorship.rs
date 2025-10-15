//! Transfer the operatorship of the Governance from the current operator to a
//! new operator.
//!
//! It can be executed only by the current operator or the program root PDA. See original implementation [here](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L96).

use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::{account_array_structs, pda::ValidPDA, validate_system_account_key};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;

use super::ensure_valid_governance_root_pda;
use crate::events;
use crate::state::GovernanceConfig;

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    TransferOperatorshipInfo,
    // Struct whose attributes are of type `AccountMeta`
    TransferOperatorshipMeta,
    // Attributes
    system_account,
    operator_account,
    config_pda,
    event_cpi_authority,
    event_cpi_program_account
}

/// Transfer the operatorship of the Governance from the current operator to a
/// new operator by altering the operator field in the [`GovernanceConfig`]
/// account.
///
/// Only the current operator or the program root PDA via a scheduled proposal
/// (self CPI call) can execute this command.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    accounts: &[AccountInfo<'_>],
    new_operator: [u8; 32],
) -> Result<(), ProgramError> {
    let TransferOperatorshipInfo {
        system_account,
        operator_account,
        config_pda,
        event_cpi_authority,
        event_cpi_program_account,
    } = TransferOperatorshipInfo::from_account_iter(&mut accounts.iter())?;
    let event_cpi_accounts = &mut [event_cpi_authority, event_cpi_program_account].into_iter();
    event_cpi_accounts!(event_cpi_accounts);

    validate_system_account_key(system_account.key)?;

    let mut config_data = config_pda.check_initialized_pda::<GovernanceConfig>(&crate::id())?;

    ensure_valid_governance_root_pda(config_data.bump, config_pda.key)?;

    if !(operator_account.is_signer || config_pda.is_signer) {
        msg!("The operator account or program root account, must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if operator_account.is_signer && operator_account.key.to_bytes() != config_data.operator {
        msg!("Operator account must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }
    let old_operator = config_data.operator;
    config_data.operator = new_operator;

    let mut data = config_pda.try_borrow_mut_data()?;
    config_data.pack_into_slice(&mut data);

    emit_cpi!(events::OperatorshipTransferred {
        old_operator,
        new_operator,
    });

    Ok(())
}
