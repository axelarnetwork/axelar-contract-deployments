//! Executes all logic for processing a `ExecuteOperatorProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L75).
use borsh::to_vec;
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::{account_array_structs, pda::ValidPDA, validate_system_account_key};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::events;
use crate::state::proposal::{ExecutableProposal, ExecuteProposalData};
use crate::state::{operator, GovernanceConfig};

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    ExecuteOperatorProposalInfo,
    // Struct whose attributes are of type `AccountMeta`
    ExecuteOperatorProposalMeta,
    // Attributes
    system_account,
    config_pda,
    proposal_account,
    operator_account,
    operator_pda_marker_account,
    event_cpi_authority,
    event_cpi_program_account
}

/// Executes a previously proposal whitelisted for execution by the operator.
///
/// # Errors
///
/// This function will return a [`ProgramError`] if any of the subcmds fail.
pub(crate) fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    execute_proposal_data: &ExecuteProposalData,
) -> Result<(), ProgramError> {
    let ExecuteOperatorProposalInfo {
        system_account,
        config_pda,
        proposal_account,
        operator_account,
        operator_pda_marker_account,
        event_cpi_authority,
        event_cpi_program_account,
    } = ExecuteOperatorProposalInfo::from_account_iter(&mut accounts.iter())?;

    let event_cpi_accounts = &mut [event_cpi_authority, event_cpi_program_account].into_iter();
    event_cpi_accounts!(event_cpi_accounts);

    validate_system_account_key(system_account.key)?;

    let config_data = config_pda.check_initialized_pda::<GovernanceConfig>(&crate::id())?;

    // Only the operator account can execute the proposal.
    if !operator_account.is_signer || operator_account.key.to_bytes() != config_data.operator {
        msg!("Operator account must sign the transaction");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let hash = ExecutableProposal::calculate_hash(
        &Pubkey::new_from_array(execute_proposal_data.target_address),
        &execute_proposal_data.call_data,
        &execute_proposal_data.native_value,
    );

    operator::ensure_correct_managed_proposal_pda(
        proposal_account,
        operator_pda_marker_account,
        &hash,
    )?;

    // Check that the proposal is executable by the operator by checking the operator_pda_marker_account
    // account is initialized.
    if !operator_pda_marker_account.is_initialized_pda(&crate::id()) {
        msg!("Operator has no approval rights for this proposal");
        return Err(ProgramError::UninitializedAccount);
    }

    let proposal = ExecutableProposal::load_from(program_id, proposal_account)?;

    // Only invoke with target program accounts.
    let mut target_program_accounts = accounts
        .get(4..)
        .ok_or(ProgramError::InvalidInstructionData)?
        .as_ref()
        .to_vec();
    target_program_accounts.push(config_pda.clone());

    proposal.unchecked_execute(
        &target_program_accounts,
        config_pda,
        config_data.bump,
        Pubkey::new_from_array(execute_proposal_data.target_address),
        execute_proposal_data.call_data.clone(),
        execute_proposal_data.find_target_native_value_account_info(accounts),
        execute_proposal_data.native_value()?,
    )?;

    // Send event
    emit_cpi!(events::OperatorProposalExecuted {
        hash,
        target_address: execute_proposal_data.target_address,
        call_data: to_vec(&execute_proposal_data.call_data).expect("Should serialize call data"),
        native_value: execute_proposal_data.native_value,
    });

    ExecutableProposal::remove(proposal_account, config_pda)?;
    program_utils::pda::close_pda(config_pda, operator_pda_marker_account, &crate::id())
}
