//! Logic for executing a proposal.Anyone can execute a proposal if the proposal
//! has reached its ETA.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/InterchainGovernance.sol#L98).
use crate::events::GovernanceEvent;
use crate::state::proposal::{ExecutableProposal, ExecuteProposalData};
use crate::state::GovernanceConfig;
use borsh::to_vec;
use program_utils::{from_u64_to_u256_le_bytes, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Executes a previously GMP received proposal if the proposal has reached its
/// ETA.
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
    let payer = next_account_info(accounts_iter)?;
    let config_pda = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    let config_data = config_pda.check_initialized_pda::<GovernanceConfig>(&crate::id())?;

    // Ensure the provided PDA matches the one obtained from the proposal data hash.
    //let hash = ensure_valid_proposal_pda(execute_proposal_data,
    // proposal_account)?;

    let hash = ExecutableProposal::calculate_hash(
        &Pubkey::new_from_array(execute_proposal_data.target_address),
        &execute_proposal_data.call_data,
        &execute_proposal_data.native_value,
    );

    ExecutableProposal::load_and_ensure_correct_proposal_pda(proposal_account, &hash)?;

    let proposal = ExecutableProposal::load_from(program_id, proposal_account)?;

    // Only invoke with target program accounts.
    let mut target_program_accounts = accounts
        .get(4..)
        .ok_or(ProgramError::InvalidInstructionData)?
        .as_ref()
        .to_vec();
    target_program_accounts.push(config_pda.clone());

    proposal.checked_execute(
        &target_program_accounts,
        config_pda,
        config_data.bump,
        Pubkey::new_from_array(execute_proposal_data.target_address),
        execute_proposal_data.call_data.clone(),
        execute_proposal_data.find_target_native_value_account_info(accounts),
        execute_proposal_data.native_value()?,
    )?;

    // Send event
    let event = GovernanceEvent::ProposalExecuted {
        hash,
        target_address: execute_proposal_data.target_address,
        call_data: to_vec(&execute_proposal_data.call_data).expect("Should serialize call data"),
        native_value: execute_proposal_data.native_value,
        // Todo: Maybe we should adopt this U256 type for the ETA field in the event.
        // Or just cast a u64 in a [u8;32] little endian.
        eta: from_u64_to_u256_le_bytes(proposal.eta()),
    };
    event.emit()?;
    ExecutableProposal::remove(proposal_account, payer)
}
