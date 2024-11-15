//! Holds all logic for processing a `CancelTimeLockProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L18).

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;

use super::ProcessGMPContext;
use crate::events::GovernanceEvent;
use crate::state::proposal::{ArchivedExecutableProposal, ExecutableProposal};

/// Processes a Governance GMP `CancelTimeLockProposal` command.
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
    let proposal_pda = next_account_info(accounts_iter)?;

    let bump = ctx.execute_proposal_call_data.proposal_bump()?;

    ExecutableProposal::ensure_correct_proposal_pda(proposal_pda.key, &ctx.proposal_hash, bump)?;

    // Check the proposal PDA exists and is initialized.
    if !proposal_pda.is_initialized_pda(&crate::id()) {
        msg!("Proposal PDA is not initialized");
        return Err(ProgramError::InvalidArgument);
    }

    ArchivedExecutableProposal::remove(proposal_pda, root_pda)?;

    // Send event
    let event = GovernanceEvent::ProposalCancelled {
        hash: ctx.proposal_hash,
        target_address: ctx.target.to_bytes(),
        call_data: ctx.cmd_payload.call_data.into(),
        native_value: ctx.cmd_payload.native_value.to_le_bytes(),
        eta: ctx.cmd_payload.eta.to_le_bytes(),
    };
    event.emit()
}
