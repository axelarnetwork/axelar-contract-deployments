//! Holds all logic for processing a governance GMP `ApproveOperatorProposal`
//! command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L17).

use program_utils::pda::ValidPDA;
use program_utils::{account_array_structs, validate_system_account_key};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::ProcessGMPContext;
use crate::events::GovernanceEvent;
use crate::seed_prefixes;
use crate::state::operator;

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    ApproveOperatorProposalInfo,
    // Struct whose attributes are of type `AccountMeta`
    ApproveOperatorProposalMeta,
    // Attributes
    // Mandatory for every GMP instruction in the Governance program.
    system_account,
    // Mandatory for every GMP instruction in the Governance program.
    #[allow(dead_code)]
    root_pda,
    payer,
    proposal_pda,
    operator_proposal_pda
}

/// Processes a Governance GMP `ApproveOperatorProposal` command.
/// After the operator proposal management is approved by the governance, the
/// operator proposal can freely execute the proposal, regardless of the
/// proposal ETA.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    ctx: ProcessGMPContext,
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
) -> Result<(), ProgramError> {
    let ApproveOperatorProposalInfo {
        system_account,
        payer,
        // Validated by the `ProcessGMPContext`, not needed here.
        root_pda: _,
        proposal_pda,
        operator_proposal_pda,
    } = ApproveOperatorProposalInfo::from_account_iter(&mut accounts.iter())?;

    validate_system_account_key(system_account.key)?;

    let bump = operator::ensure_correct_managed_proposal_pda(
        proposal_pda,
        operator_proposal_pda,
        &ctx.proposal_hash,
    )?;

    if operator_proposal_pda.is_initialized_pda(&crate::ID) {
        msg!("Proposal already under operator control");
        return Err(ProgramError::InvalidArgument);
    }

    program_utils::pda::init_pda_raw_bytes(
        payer,
        operator_proposal_pda,
        program_id,
        system_account,
        &[1], // We store a single non-zero byte to indicate initialization
        &[
            seed_prefixes::OPERATOR_MANAGED_PROPOSAL,
            &ctx.proposal_hash,
            &[bump],
        ],
    )?;

    // Send event
    let event = GovernanceEvent::OperatorProposalApproved {
        hash: ctx.proposal_hash,
        target_address: ctx.target.to_bytes(),
        call_data: ctx.cmd_payload.call_data.into(),
        native_value: ctx.cmd_payload.native_value.to_le_bytes(),
    };

    event.emit()
}
