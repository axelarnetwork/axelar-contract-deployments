//! Holds all logic for processing a `CancelOperatorApproval` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L17).

use program_utils::{account_array_structs, validate_system_account_key};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;

use super::ProcessGMPContext;
use crate::events::GovernanceEvent;
use crate::state::operator;

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    CancelOperatorApprovalInfo,
    // Struct whose attributes are of type `AccountMeta`
    CancelOperatorApprovalMeta,
    // Attributes
    // Mandatory for every GMP instruction in the Governance program.
    system_account,
    // Mandatory for every GMP instruction in the Governance program.
    root_pda,
    proposal_pda,
    operator_proposal_pda
}

/// Processes a Governance GMP `CancelOperatorApproval` command.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    ctx: ProcessGMPContext,
    accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    let CancelOperatorApprovalInfo {
        system_account,
        root_pda,
        proposal_pda,
        operator_proposal_pda,
    } = CancelOperatorApprovalInfo::from_account_iter(&mut accounts.iter())?;

    validate_system_account_key(system_account.key)?;

    operator::ensure_correct_managed_proposal_pda(
        proposal_pda,
        operator_proposal_pda,
        &ctx.proposal_hash,
    )?;

    program_utils::pda::close_pda(root_pda, operator_proposal_pda)?;

    // Send event
    let event = GovernanceEvent::OperatorProposalCancelled {
        hash: ctx.proposal_hash,
        target_address: ctx.target.to_bytes(),
        call_data: ctx.cmd_payload.call_data.into(),
        native_value: ctx.cmd_payload.native_value.to_le_bytes(),
    };

    event.emit()
}
