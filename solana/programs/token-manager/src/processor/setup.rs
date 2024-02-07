//! Setup a new token manager

use account_group::state::{PermissionAccount, PermissionGroupAccount};
use program_utils::{check_program_account, init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::{
    assert_permission_group_pda, assert_permission_pda, assert_token_manager_account, Processor,
};
use crate::check_id;
use crate::state::TokenManagerRootAccount;

impl Processor {
    /// Sets up a new Token Manager with the provided parameters.
    pub fn process_setup(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        flow_limit: u64,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let token_manager_root_pda = next_account_info(account_info_iter)?;
        let operators_permission_group_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda = next_account_info(account_info_iter)?;
        let operators_permission_pda_owner = next_account_info(account_info_iter)?;
        let flow_limiters_permission_group_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda_owner = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        // Assert account groups
        let operator_group = operators_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(&account_group::ID)?
            .id;
        let flow_group = flow_limiters_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(&account_group::ID)?
            .id;
        let _perm_pda = operators_permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)?;
        let _perm_pda = flow_limiters_permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)?;
        assert_permission_group_pda(operator_group, operators_permission_group_pda);
        assert_permission_group_pda(flow_group, flow_limiters_permission_group_pda);
        assert_permission_pda(
            operators_permission_group_pda,
            operators_permission_pda,
            operators_permission_pda_owner,
        );
        assert_permission_pda(
            flow_limiters_permission_group_pda,
            flow_limiters_permission_pda,
            flow_limiters_permission_pda_owner,
        );

        // Assert token manager pdas
        token_manager_root_pda.check_uninitialized_pda()?;

        let token_manager_root_pda_bump = assert_token_manager_account(
            token_manager_root_pda,
            operators_permission_group_pda,
            flow_limiters_permission_group_pda,
            service_program_pda,
            program_id,
        )?;

        // Initialize PDAs
        init_pda(
            funder_info,
            token_manager_root_pda,
            program_id,
            system_program,
            TokenManagerRootAccount { flow_limit },
            &[
                &operators_permission_group_pda.key.to_bytes(),
                &flow_limiters_permission_group_pda.key.to_bytes(),
                &service_program_pda.key.to_bytes(),
                &[token_manager_root_pda_bump],
            ],
        )?;

        Ok(())
    }
}
