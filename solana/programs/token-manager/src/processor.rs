//! Program state processor

use account_group::get_permission_group_account;
use account_group::instruction::GroupId;
use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instruction::TokenManagerInstruction;
use crate::{
    get_token_flow_account_and_bump_seed_internal,
    get_token_manager_account_and_bump_seed_internal, CalculatedEpoch,
};

mod add_flow;
mod set_flow_limit;
mod setup;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = TokenManagerInstruction::try_from_slice(input)?;

        match instruction {
            TokenManagerInstruction::Setup(setup) => {
                Processor::process_setup(program_id, accounts, setup)
            }
            TokenManagerInstruction::SetFlowLimit { amount } => {
                Processor::process_set_flow_limit(program_id, accounts, amount)
            }
            TokenManagerInstruction::AddFlowDirection(flow_addition) => {
                Processor::update_flows(program_id, accounts, flow_addition)
            }
        }
    }
}

fn assert_token_manager_account(
    token_manager_root_pda: &AccountInfo<'_>,
    operators_permission_group_pda: &AccountInfo<'_>,
    flow_limiters_permission_group_pda: &AccountInfo<'_>,
    service_program_pda: &AccountInfo<'_>,
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let (derived_account, bump_seed) = get_token_manager_account_and_bump_seed_internal(
        operators_permission_group_pda.key,
        flow_limiters_permission_group_pda.key,
        service_program_pda.key,
        program_id,
    );
    if derived_account != *token_manager_root_pda.key {
        msg!("Error: Provided address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}

fn assert_permission_group_pda(
    permission_group_id: GroupId,
    operators_permission_group_pda: &AccountInfo<'_>,
) {
    // Assert that the permission group account is derived from the permission group
    // id
    let derived_operators_permission_group_pda = get_permission_group_account(&permission_group_id);
    assert_eq!(
        derived_operators_permission_group_pda, *operators_permission_group_pda.key,
        "permission group account is not derived from permission group id"
    );
}

fn assert_flow_limit_pda(
    token_manager_root_pda: &AccountInfo<'_>,
    flow_limit_pda: &AccountInfo<'_>,
    program_id: &Pubkey,
    epoch: CalculatedEpoch,
) -> Result<u8, ProgramError> {
    let (derived_account, bump) = get_token_flow_account_and_bump_seed_internal(
        token_manager_root_pda.key,
        epoch,
        program_id,
    );

    assert_eq!(
        derived_account, *flow_limit_pda.key,
        "Flow limit account is not derived from token manager account and flow limit"
    );

    Ok(bump)
}

fn assert_permission_pda(
    operators_permission_group_pda: &AccountInfo<'_>,
    permission_pda: &AccountInfo<'_>,
    permission_pda_owner: &AccountInfo<'_>,
) {
    let derived_account = account_group::get_permission_account(
        operators_permission_group_pda.key,
        permission_pda_owner.key,
    );
    assert_eq!(
        derived_account, *permission_pda.key,
        "permission account is not derived from permission group account and permission account owner"
    );
}
