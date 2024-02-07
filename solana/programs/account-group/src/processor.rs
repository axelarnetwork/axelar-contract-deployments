//! Program state processor

mod add_account_to_group;
mod create_group;
mod renounce;

use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instruction::{GroupId, PermissionGroupInstruction};
use crate::{
    get_permission_account_and_bump_seed_internal,
    get_permission_group_account_and_bump_seed_internal,
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
        let instruction = PermissionGroupInstruction::try_from_slice(input)?;

        match instruction {
            PermissionGroupInstruction::SetupPermissionGroup { id } => {
                msg!("SetupPermissionGroup");
                Self::process_create_group(program_id, accounts, id)
            }
            PermissionGroupInstruction::AddAccountToPermissionGroup => {
                msg!("AddAccountToPermissionGroup");
                Self::process_add_account_to_group(program_id, accounts)
            }
            PermissionGroupInstruction::RenouncePermission => {
                msg!("RenouncePermission");
                Self::process_renounce(program_id, accounts)
            }
        }
    }
}

fn assert_permission_group_account(
    permissions_group_pda: &AccountInfo<'_>,
    program_id: &Pubkey,
    id: &GroupId,
) -> Result<u8, ProgramError> {
    let (derived_account, bump_seed) =
        get_permission_group_account_and_bump_seed_internal(id, program_id);
    if derived_account != *permissions_group_pda.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}
fn assert_permission_account(
    permission_pda: &AccountInfo<'_>,
    permission_group_pda: &AccountInfo<'_>,
    permission_owner_account: &AccountInfo<'_>,
    program_id: &Pubkey,
) -> Result<u8, ProgramError> {
    let (derived_account, bump_seed) = get_permission_account_and_bump_seed_internal(
        permission_group_pda.key,
        permission_owner_account.key,
        program_id,
    );
    if derived_account != *permission_pda.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}
