use program_utils::{check_program_account, close_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::{assert_permission_account, assert_permission_group_account, Processor};
use crate::check_id;
use crate::state::{PermissionAccount, PermissionGroupAccount};

impl Processor {
    /// Renounce a permission
    pub fn process_renounce(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let existing_permission_group_pda = next_account_info(account_info_iter)?;
        let existing_permission_pda = next_account_info(account_info_iter)?;
        let existing_permission_owner_account = next_account_info(account_info_iter)?;

        let group = existing_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(program_id)?;
        let _ = existing_permission_pda.check_initialized_pda::<PermissionAccount>(program_id)?;
        let _bump_seed =
            assert_permission_group_account(existing_permission_group_pda, program_id, &group.id)?;
        let _bump_seed = assert_permission_account(
            existing_permission_pda,
            existing_permission_group_pda,
            existing_permission_owner_account,
            program_id,
        )?;

        close_pda(existing_permission_owner_account, existing_permission_pda)?;

        Ok(())
    }
}
