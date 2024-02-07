//! Add an permission to an permission group

use program_utils::{check_program_account, init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::check_id;
use crate::processor::{assert_permission_account, assert_permission_group_account};
use crate::state::{PermissionAccount, PermissionGroupAccount};

impl Processor {
    /// Add an permission to an permission group
    pub fn process_add_account_to_group(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let existing_permission_group_pda = next_account_info(account_info_iter)?;
        let existing_permission_pda = next_account_info(account_info_iter)?;
        let existing_permission_owner_account = next_account_info(account_info_iter)?;
        let new_permission_owner_account = next_account_info(account_info_iter)?;
        let new_permission_pda = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let group = existing_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(program_id)?;
        let _ = existing_permission_pda.check_initialized_pda::<PermissionAccount>(program_id)?;
        let _ = new_permission_pda.check_uninitialized_pda();
        let _ =
            assert_permission_group_account(existing_permission_group_pda, program_id, &group.id)?;
        assert!(
            existing_permission_owner_account.is_signer,
            "Existing permission account must be signer"
        );
        // Make sure existing permission account is associated with the permission group
        // account
        let _ = assert_permission_account(
            existing_permission_pda,
            existing_permission_group_pda,
            existing_permission_owner_account,
            program_id,
        )?;
        // Create a new permission account
        let bump_seed = assert_permission_account(
            new_permission_pda,
            existing_permission_group_pda,
            new_permission_owner_account,
            program_id,
        )?;

        // Create a new PDA
        init_pda(
            funder_info,
            new_permission_pda,
            program_id,
            system_program_info,
            PermissionAccount,
            &[
                &existing_permission_group_pda.key.as_ref(),
                &new_permission_owner_account.key.as_ref(),
                &[bump_seed],
            ],
        )?;
        Ok(())
    }
}
