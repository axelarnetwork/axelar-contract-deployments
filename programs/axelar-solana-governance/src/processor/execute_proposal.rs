//! Logic for executing a proposal.Anyone can execute a proposal if the proposal
//! has reached its ETA.
//!
//! See [original implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/governance/InterchainGovernance.sol#L98).
use crate::events;
use crate::state::proposal::{ExecutableProposal, ExecuteProposalData};
use crate::state::GovernanceConfig;
use borsh::to_vec;
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::{
    account_array_structs, from_u64_to_u256_le_bytes, pda::ValidPDA, validate_system_account_key,
};
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

account_array_structs! {
    // Struct whose attributes are of type `AccountInfo`
    ExecuteProposalInfo,
    // Struct whose attributes are of type `AccountMeta`
    ExecuteProposalMeta,
    // Attributes
    system_account,
    config_pda,
    proposal_account,
    event_cpi_authority,
    event_cpi_program_account
}

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
    let ExecuteProposalInfo {
        system_account,
        config_pda,
        proposal_account,
        event_cpi_authority,
        event_cpi_program_account,
    } = ExecuteProposalInfo::from_account_iter(&mut accounts.iter())?;

    let event_cpi_accounts = &mut [event_cpi_authority, event_cpi_program_account].into_iter();
    event_cpi_accounts!(event_cpi_accounts);

    validate_system_account_key(system_account.key)?;

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
        .get(3..)
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
    emit_cpi!(events::ProposalExecuted {
        hash,
        target_address: execute_proposal_data.target_address,
        call_data: to_vec(&execute_proposal_data.call_data).expect("Should serialize call data"),
        native_value: execute_proposal_data.native_value,
        // Todo: Maybe we should adopt this U256 type for the ETA field in the event.
        // Or just cast a u64 in a [u8;32] little endian.
        eta: from_u64_to_u256_le_bytes(proposal.eta()),
    });
    ExecutableProposal::remove(proposal_account, config_pda)
}
