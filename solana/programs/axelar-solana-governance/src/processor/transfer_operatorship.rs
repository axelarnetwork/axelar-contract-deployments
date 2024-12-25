//! Transfer the operatorship of the Governance from the current operator to a
//! new operator.
//!
//! It can be executed only by the current operator or the program root PDA. See original implementation [here](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L96).

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::ensure_valid_governance_root_pda;
use crate::events::GovernanceEvent;
use crate::state::GovernanceConfig;

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
    _program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    new_operator: [u8; 32],
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let _payer = next_account_info(accounts_iter)?;
    let operator_account = next_account_info(accounts_iter)?;
    let config_pda = next_account_info(accounts_iter)?;

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

    let event = GovernanceEvent::OperatorshipTransferred {
        old_operator,
        new_operator,
    };
    event.emit()
}
