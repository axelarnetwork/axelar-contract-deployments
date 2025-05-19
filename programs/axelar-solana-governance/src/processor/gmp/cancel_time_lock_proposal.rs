//! Holds all logic for processing a `CancelTimeLockProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L18).

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;

use super::ProcessGMPContext;
use crate::events::GovernanceEvent;
use crate::state::proposal::ExecutableProposal;

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

    ExecutableProposal::load_and_ensure_correct_proposal_pda(proposal_pda, &ctx.proposal_hash)?;

    ExecutableProposal::remove(proposal_pda, root_pda)?;

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
