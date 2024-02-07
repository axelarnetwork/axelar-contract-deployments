//! Set a new flow limit for a given token manager.

use account_group::state::{PermissionAccount, PermissionGroupAccount};
use borsh::to_vec;
use program_utils::{check_program_account, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::{assert_permission_pda, assert_token_manager_account, Processor};
use crate::check_id;
use crate::processor::assert_permission_group_pda;
use crate::state::TokenManagerRootAccount;

impl Processor {
    /// Sets the flow limit for a given token manager.
    ///
    /// This function is responsible for setting the flow limit of a token
    /// manager. The flow limit is a parameter that controls the maximum
    /// amount of tokens that can be transferred from the token manager per unit
    /// of time.
    pub fn process_set_flow_limit(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        flow_limit: u64,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let token_manager_root_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_group_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda_owner = next_account_info(account_info_iter)?;
        let operators_permission_group_pda = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;

        // Assert account groups
        let flow_group = flow_limiters_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(&account_group::ID)?
            .id;
        let _perm_pda = flow_limiters_permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)?;
        assert_permission_group_pda(flow_group, flow_limiters_permission_group_pda);
        assert_permission_pda(
            flow_limiters_permission_group_pda,
            flow_limiters_permission_pda,
            flow_limiters_permission_pda_owner,
        );

        // Make sure that only someone with the flow limiters permission can set the
        // flow limit
        assert!(
            flow_limiters_permission_pda_owner.is_signer,
            "Flow limiter must be signer"
        );

        // Assert token manager account
        let _token_manager_root_pda_bump = assert_token_manager_account(
            token_manager_root_pda,
            operators_permission_group_pda,
            flow_limiters_permission_group_pda,
            service_program_pda,
            program_id,
        )?;
        let mut data =
            token_manager_root_pda.check_initialized_pda::<TokenManagerRootAccount>(program_id)?;

        // Set the flow limit
        data.flow_limit = flow_limit;
        let serialized_data = to_vec(&data)?;
        let mut account_data = token_manager_root_pda.try_borrow_mut_data()?;
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);

        Ok(())
    }
}
