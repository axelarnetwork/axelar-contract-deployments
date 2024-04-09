//! Create a new group

use program_utils::{check_program_account, init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::{assert_permission_account, assert_permission_group_account, Processor};
use crate::check_id;
use crate::instruction::GroupId;
use crate::state::{PermissionAccount, PermissionGroupAccount};

impl Processor {
    /// Create a new group
    pub fn process_create_group(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        id: GroupId,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let permission_group_account = next_account_info(account_info_iter)?;
        let permission_account = next_account_info(account_info_iter)?;
        let permission_account_owner = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        // Setup Permission Group
        permission_group_account.check_uninitialized_pda()?;
        let bump_seed = assert_permission_group_account(permission_group_account, program_id, &id)?;
        init_pda(
            funder_info,
            permission_group_account,
            program_id,
            system_program_info,
            PermissionGroupAccount::new(id.clone()),
            &[&id.to_bytes(), &[bump_seed]],
        )?;

        // Setup Permissioned Account
        permission_account.check_uninitialized_pda()?;
        let bump_seed = assert_permission_account(
            permission_account,
            permission_group_account,
            permission_account_owner,
            program_id,
        )?;
        init_pda(
            funder_info,
            permission_account,
            program_id,
            system_program_info,
            PermissionAccount,
            &[
                (permission_group_account.key.as_ref()),
                (permission_account_owner.key.as_ref()),
                &[bump_seed],
            ],
        )?;

        Ok(())
    }
}
