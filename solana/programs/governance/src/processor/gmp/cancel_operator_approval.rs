//! Holds all logic for processing a `CancelOperatorApproval` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L17).

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;

use super::ProcessGMPContext;
use crate::events::GovernanceEvent;
use crate::state::operator;

/// Processes a Governance GMP `CancelOperatorApproval` command.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    ctx: ProcessGMPContext,
    accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let _payer = next_account_info(accounts_iter)?;
    let root_pda = next_account_info(accounts_iter)?;
    let operator_proposal_pda = next_account_info(accounts_iter)?;

    let bump = ctx
        .execute_proposal_call_data
        .proposal_operator_managed_bump()?;

    operator::ensure_correct_managed_proposal_pda(
        operator_proposal_pda.key,
        &ctx.proposal_hash,
        bump,
    )?;

    program_utils::close_pda(root_pda, operator_proposal_pda)?;

    // Send event
    let event = GovernanceEvent::OperatorProposalCancelled {
        hash: ctx.proposal_hash,
        target_address: ctx.target.to_bytes(),
        call_data: ctx.cmd_payload.call_data.into(),
        native_value: ctx.cmd_payload.native_value.to_le_bytes(),
    };

    event.emit()
}
