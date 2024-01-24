//! Set a new flow limit for a given token manager.

use borsh::{to_vec, BorshDeserialize};
use program_utils::{check_program_account, init_pda};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use super::{assert_operator_pda, assert_token_manager_account, Processor};
use crate::instruction::FlowToAdd;
use crate::processor::assert_flow_limit_pda_account;
use crate::state::{FlowInOutAccount, TokenManagerAccount};
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
        let token_manager_pda = next_account_info(account_info_iter)?;
        let flow_limit_pda = next_account_info(account_info_iter)?;
        let flow_limiter_group_pda = next_account_info(account_info_iter)?;
        let flow_limiter_pda = next_account_info(account_info_iter)?;
        let flow_limiter = next_account_info(account_info_iter)?;
        let operator_group_pda = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        let epoch = CalculatedEpoch::new();

        assert_eq!(flow_limiter_group_pda.owner, &operator::ID);
        assert!(flow_limiter.is_signer, "Flow limiter must be signer");
        assert!(funder_info.is_signer, "Funder must be signer");
        assert_operator_pda(flow_limiter_group_pda, flow_limiter_pda, flow_limiter);

        let _bump_seed = assert_token_manager_account(
            token_manager_pda,
            operator_group_pda,
            flow_limiter_group_pda,
            service_program_pda,
            program_id,
        )?;
        assert_eq!(
            &program_id, &token_manager_pda.owner,
            "Token Manager PDA needs to be initialized and owned by the Token Manager program"
        );

        let flow_limit_pda_bump =
            assert_flow_limit_pda_account(token_manager_pda, flow_limit_pda, program_id, epoch)?;

        let data = token_manager_pda.try_borrow_data()?;
        let token_manager_data = TokenManagerAccount::try_from_slice(&data[..data.len()])?;
        if flow_limit_pda.owner == &system_program::id() {
            check_flow_amounts(
                token_manager_data.flow_limit,
                &FlowInOutAccount {
                    flow_in: 0,
                    flow_out: 0,
                },
                &flow_addition,
            );
            let data = FlowInOutAccount {
                flow_in: flow_addition.add_flow_in,
                flow_out: flow_addition.add_flow_out,
            };
            msg!("Creating flow limit PDA {:?}", data);
            init_pda(
                funder_info,
                flow_limit_pda,
                program_id,
                system_program,
                data,
                &[
                    &token_manager_pda.key.to_bytes(),
                    &epoch.to_le_bytes(),
                    &[flow_limit_pda_bump],
                ],
            )?;
        } else if flow_limit_pda.owner == &crate::id() {
            let mut account_data = flow_limit_pda.try_borrow_mut_data()?;
            let mut data = FlowInOutAccount::try_from_slice(&account_data[..account_data.len()])?;
            check_flow_amounts(token_manager_data.flow_limit, &data, &flow_addition);
            data.flow_in += flow_addition.add_flow_in;
            data.flow_out += flow_addition.add_flow_out;
            msg!("Updating flow limit PDA - new limit: {:?}", data);
            let serialized_data = to_vec(&data).unwrap();
            account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        } else {
            return Err(ProgramError::IllegalOwner);
        }

        Ok(())
    }
}

// https://github.com/axelarnetwork/interchain-token-service/blob/0bcd568c7c1814e18694b31c0f0980798d79548a/contracts/utils/FlowLimit.sol#L94
fn check_flow_amounts(flow_limit: u64, current: &FlowInOutAccount, new_flow_to_add: &FlowToAdd) {
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
