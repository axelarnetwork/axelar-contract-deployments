//! Program state processor

use borsh::BorshDeserialize;
use operator::get_operator_group_account;
use operator::state::OperatorAccount;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instruction::{Setup, TokenManagerInstruction};
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
            TokenManagerInstruction::Setup(Setup {
                operator_group_id,
                flow_limiter_group_id,
                flow_limit,
            }) => Processor::process_setup(
                program_id,
                accounts,
                operator_group_id,
                flow_limiter_group_id,
                flow_limit,
            ),
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
    token_manager_pda: &AccountInfo<'_>,
    operator_group_pda: &AccountInfo<'_>,
    flow_limiter_group_pda: &AccountInfo<'_>,
    service_program_pda: &AccountInfo<'_>,
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let (derived_account, bump_seed) = get_token_manager_account_and_bump_seed_internal(
        operator_group_pda.key,
        flow_limiter_group_pda.key,
        service_program_pda.key,
        program_id,
    );
    if derived_account != *token_manager_pda.key {
        msg!("Error: Provided address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}

fn assert_operator_group_pda(operator_group_id: String, operator_group_pda: &AccountInfo<'_>) {
    // Assert that the operator group account is derived from the operator group id
    let derived_operator_group_pda = get_operator_group_account(operator_group_id.as_str());
    assert_eq!(
        derived_operator_group_pda, *operator_group_pda.key,
        "Operator group account is not derived from operator group id"
    );
    assert_eq!(
        &operator::ID,
        operator_group_pda.owner,
        "Operator group account is not owned by the operator program"
    );
}

fn assert_flow_limit_pda_account(
    token_manager_pda: &AccountInfo<'_>,
    flow_limit_pda: &AccountInfo<'_>,
    program_id: &Pubkey,
    epoch: CalculatedEpoch,
) -> Result<u8, ProgramError> {
    let (derived_account, bump) =
        get_token_flow_account_and_bump_seed_internal(token_manager_pda.key, epoch, program_id);
    assert_eq!(
        derived_account, *flow_limit_pda.key,
        "Flow limit account is not derived from token manager account and flow limit"
    );

    Ok(bump)
}

fn assert_operator_pda(
    operator_group_pda: &AccountInfo<'_>,
    operator_pda: &AccountInfo<'_>,
    operator: &AccountInfo<'_>,
) {
    let derived_account = operator::get_operator_account(operator_group_pda.key, operator.key);
    assert_eq!(
        derived_account, *operator_pda.key,
        "Operator account is not derived from operator group account and operator account owner"
    );
    assert_eq!(
        &operator::ID,
        operator_pda.owner,
        "Operator account is not owned by the operator program"
    );
    let account_data = operator_pda
        .try_borrow_data()
        .expect("Failed to borrow data");
    let data = OperatorAccount::try_from_slice(&account_data[..account_data.len()])
        .expect("Failed to deserialize data");
    assert!(data.is_active(), "Operator account is not active");
}
