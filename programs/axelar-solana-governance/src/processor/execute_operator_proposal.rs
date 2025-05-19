//! Executes all logic for processing a `ExecuteOperatorProposal` command.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/AxelarServiceGovernance.sol#L75).
use borsh::to_vec;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::events::GovernanceEvent;
use crate::state::proposal::{ExecutableProposal, ExecuteProposalData};
use crate::state::{operator, GovernanceConfig};

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
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let _payer = next_account_info(accounts_iter)?;
    let config_pda = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;
    let operator_account = next_account_info(accounts_iter)?;
    let operator_pda_marker_account = next_account_info(accounts_iter)?;

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
    let event = GovernanceEvent::OperatorProposalExecuted {
        hash,
        target_address: execute_proposal_data.target_address,
        call_data: to_vec(&execute_proposal_data.call_data).expect("Should serialize call data"),
        native_value: execute_proposal_data.native_value,
    };
    event.emit()?;
    ExecutableProposal::remove(proposal_account, config_pda)?;
    program_utils::close_pda(config_pda, operator_pda_marker_account)
}
