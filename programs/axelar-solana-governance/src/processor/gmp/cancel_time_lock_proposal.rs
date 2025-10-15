//! Holds all logic for processing a `CancelTimeLockProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L18).

use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::{account_array_structs, validate_system_account_key};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;

use super::ProcessGMPContext;
use crate::events;
use crate::state::proposal::ExecutableProposal;

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    CancelTimeLockProposalInfo,
    // Struct whose attributes are of type `AccountMeta`
    CancelTimeLockProposalMeta,
    // Attributes
    // Mandatory for every GMP instruction in the Governance program.
    system_account,
    // Mandatory for every GMP instruction in the Governance program.
    root_pda,
    proposal_pda,
    event_cpi_authority,
    event_cpi_program_account
}

/// Processes a Governance GMP `CancelTimeLockProposal` command.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    ctx: ProcessGMPContext,
    accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    let CancelTimeLockProposalInfo {
        system_account,
        root_pda,
        proposal_pda,
        event_cpi_authority,
        event_cpi_program_account,
    } = CancelTimeLockProposalInfo::from_account_iter(&mut accounts.iter())?;

    let event_cpi_accounts = &mut [event_cpi_authority, event_cpi_program_account].into_iter();
    event_cpi_accounts!(event_cpi_accounts);

    validate_system_account_key(system_account.key)?;

    ExecutableProposal::load_and_ensure_correct_proposal_pda(proposal_pda, &ctx.proposal_hash)?;

    ExecutableProposal::remove(proposal_pda, root_pda)?;

    // Send event

    emit_cpi!(events::ProposalCancelled {
        hash: ctx.proposal_hash,
        target_address: ctx.target.to_bytes(),
        call_data: ctx.cmd_payload.call_data.into(),
        native_value: ctx.cmd_payload.native_value.to_le_bytes(),
        eta: ctx.cmd_payload.eta.to_le_bytes(),
    });

    Ok(())
}
