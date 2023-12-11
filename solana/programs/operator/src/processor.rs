//! Program state processor

mod add_operator;
mod create_group;

use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instruction::OperatorInstruction;
use crate::{
    get_operator_address_account_and_bump_seed_internal,
    get_operator_group_account_and_bump_seed_internal,
};

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = OperatorInstruction::try_from_slice(input)?;

        match instruction {
            OperatorInstruction::CreateOperatorGroup { id } => {
                Self::process_create_group(program_id, accounts, id)
            }
            OperatorInstruction::AddOperator => Self::process_add_operator(program_id, accounts),
        }
    }
}

fn assert_operator_group_account(
    operators_group_chain_account: &AccountInfo<'_>,
    program_id: &Pubkey,
    id: &str,
) -> Result<u8, ProgramError> {
    let (derived_account, bump_seed) =
        get_operator_group_account_and_bump_seed_internal(id, program_id);
    if derived_account != *operators_group_chain_account.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}
fn assert_operator_account(
    operator_pda_account: &AccountInfo<'_>,
    operator_group_pda_account: &AccountInfo<'_>,
    operator_owner_account: &AccountInfo<'_>,
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let (derived_account, bump_seed) = get_operator_address_account_and_bump_seed_internal(
        operator_group_pda_account.key,
        operator_owner_account.key,
        program_id,
    );
    if derived_account != *operator_pda_account.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}
