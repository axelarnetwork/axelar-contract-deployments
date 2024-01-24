//! Set a new flow limit for a given token manager.

use borsh::{to_vec, BorshDeserialize};
use program_utils::check_program_account;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::{assert_operator_pda, assert_token_manager_account, Processor};
use crate::check_id;
use crate::state::TokenManagerAccount;

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

        let token_manager_pda = next_account_info(account_info_iter)?;
        let flow_limiter_group_pda = next_account_info(account_info_iter)?;
        let flow_limiter_pda = next_account_info(account_info_iter)?;
        let flow_limiter = next_account_info(account_info_iter)?;
        let operator_group_pda = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;

        assert_eq!(flow_limiter_group_pda.owner, &operator::ID);
        assert_operator_pda(flow_limiter_group_pda, flow_limiter_pda, flow_limiter);

        assert!(flow_limiter.is_signer, "Flow limiter must be signer");

        let _bump_seed = assert_token_manager_account(
            token_manager_pda,
            operator_group_pda,
            flow_limiter_group_pda,
            service_program_pda,
            program_id,
        )?;
        if token_manager_pda.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        let mut account_data = token_manager_pda.try_borrow_mut_data()?;
        let mut data = TokenManagerAccount::try_from_slice(&account_data[..account_data.len()])?;
        data.flow_limit = flow_limit;
        let serialized_data = to_vec(&data)?;
        account_data[..serialized_data.len()].copy_from_slice(&serialized_data);

        Ok(())
    }
}
