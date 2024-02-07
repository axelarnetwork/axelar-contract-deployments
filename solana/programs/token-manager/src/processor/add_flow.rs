//! Set a new flow limit for a given token manager.

use account_group::state::{PermissionAccount, PermissionGroupAccount};
use borsh::to_vec;
use program_utils::{check_program_account, init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::{
    assert_permission_group_pda, assert_permission_pda, assert_token_manager_account, Processor,
};
use crate::instruction::FlowToAdd;
use crate::processor::assert_flow_limit_pda;
use crate::state::{TokenManagerFlowInOutAccount, TokenManagerRootAccount};
use crate::{check_id, CalculatedEpoch};

impl Processor {
    /// Sets the flow limit for a given token manager.
    ///
    /// This function is responsible for setting the flow limit of a token
    /// manager. The flow limit is a parameter that controls the maximum
    /// amount of tokens that can be transferred from the token manager per unit
    /// of time.
    pub fn update_flows(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        flow_addition: FlowToAdd,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let token_manager_root_pda = next_account_info(account_info_iter)?;
        let token_manager_flow_limit_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_group_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda = next_account_info(account_info_iter)?;
        let flow_limiters_permission_pda_owner = next_account_info(account_info_iter)?;
        let operators_permission_group_pda = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        let epoch = CalculatedEpoch::new();

        // Assert account groups
        let flow_group = flow_limiters_permission_group_pda
            .check_initialized_pda::<PermissionGroupAccount>(&account_group::ID)?;
        let _perm_pda = flow_limiters_permission_pda
            .check_initialized_pda::<PermissionAccount>(&account_group::ID)?;
        assert_permission_group_pda(flow_group.id, flow_limiters_permission_group_pda);
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
        let root_account_data =
            token_manager_root_pda.check_initialized_pda::<TokenManagerRootAccount>(program_id)?;

        let flow_limit_pda_bump = assert_flow_limit_pda(
            token_manager_root_pda,
            token_manager_flow_limit_pda,
            program_id,
            epoch,
        )?;

        match token_manager_flow_limit_pda
            .check_initialized_pda::<TokenManagerFlowInOutAccount>(program_id)
        {
            // Current data exists, we just need to update it
            Ok(mut flow_in_out_data) => {
                // Update the data
                check_flow_amounts(
                    root_account_data.flow_limit,
                    &flow_in_out_data,
                    &flow_addition,
                );
                flow_in_out_data.flow_in += flow_addition.add_flow_in;
                flow_in_out_data.flow_out += flow_addition.add_flow_out;

                // Write the data
                let mut account_data = token_manager_flow_limit_pda.try_borrow_mut_data()?;
                let serialized_data = to_vec(&flow_in_out_data).unwrap();
                account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
            }
            // Account does not exist - initialize it
            Err(_) => {
                check_flow_amounts(
                    root_account_data.flow_limit,
                    &TokenManagerFlowInOutAccount {
                        flow_in: 0,
                        flow_out: 0,
                    },
                    &flow_addition,
                );
                let data = TokenManagerFlowInOutAccount {
                    flow_in: flow_addition.add_flow_in,
                    flow_out: flow_addition.add_flow_out,
                };
                init_pda(
                    funder_info,
                    token_manager_flow_limit_pda,
                    program_id,
                    system_program,
                    data,
                    &[
                        &token_manager_root_pda.key.to_bytes(),
                        &epoch.to_le_bytes(),
                        &[flow_limit_pda_bump],
                    ],
                )?;
            }
        }

        Ok(())
    }
}

// https://github.com/axelarnetwork/interchain-token-service/blob/0bcd568c7c1814e18694b31c0f0980798d79548a/contracts/utils/FlowLimit.sol#L94
fn check_flow_amounts(
    flow_limit: u64,
    current: &TokenManagerFlowInOutAccount,
    new_flow_to_add: &FlowToAdd,
) {
    assert!(flow_limit != 0, "Flow limit cannot be 0");
    single_flow_data_check(
        current.flow_out,
        new_flow_to_add.add_flow_out,
        current.flow_in,
        flow_limit,
    );
    single_flow_data_check(
        current.flow_in,
        new_flow_to_add.add_flow_in,
        current.flow_out,
        flow_limit,
    );
}

fn single_flow_data_check(
    current_flow_value: u64,
    flow_to_add: u64,
    current_flow_to_compare: u64,
    flow_limit: u64,
) {
    assert!(
        (current_flow_value + flow_to_add) <= (current_flow_to_compare + flow_limit),
        "Flow limit exceeded 1"
    );
    assert!(flow_to_add <= flow_limit, "Flow limit exceeded 2");
}
