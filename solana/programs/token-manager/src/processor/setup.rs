//! Setup a new token manager

use program_utils::{check_program_account, init_pda};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::{
    assert_operator_group_pda, assert_operator_pda, assert_token_manager_account, Processor,
};
use crate::check_id;
use crate::state::TokenManagerAccount;

impl Processor {
    /// Sets up a new Token Manager with the provided parameters.
    pub fn process_setup(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        operator_group_id: String,
        flow_limiter_group_id: String,
        flow_limit: u64,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let token_manager_pda = next_account_info(account_info_iter)?;
        let operator_group_pda = next_account_info(account_info_iter)?;
        let operator_pda = next_account_info(account_info_iter)?;
        let operator = next_account_info(account_info_iter)?;
        let flow_limiter_group_pda = next_account_info(account_info_iter)?;
        let flow_limiter_pda = next_account_info(account_info_iter)?;
        let flow_limiter = next_account_info(account_info_iter)?;
        let service_program_pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        assert_operator_group_pda(operator_group_id, operator_group_pda);
        assert_operator_group_pda(flow_limiter_group_id, flow_limiter_group_pda);
        assert_operator_pda(operator_group_pda, operator_pda, operator);
        assert_operator_pda(flow_limiter_group_pda, flow_limiter_pda, flow_limiter);

        assert!(funder_info.is_signer, "Funder must be signer");
        assert!(operator.is_signer, "Operator must be signer");
        assert!(flow_limiter.is_signer, "Flow limiter must be signer");

        let bump_seed = assert_token_manager_account(
            token_manager_pda,
            operator_group_pda,
            flow_limiter_group_pda,
            service_program_pda,
            program_id,
        )?;
        if *token_manager_pda.owner != system_program::id() {
            return Err(ProgramError::IllegalOwner);
        }

        init_pda(
            funder_info,
            token_manager_pda,
            program_id,
            system_program,
            TokenManagerAccount { flow_limit },
            &[
                &operator_group_pda.key.to_bytes(),
                &flow_limiter_group_pda.key.to_bytes(),
                &service_program_pda.key.to_bytes(),
                &[bump_seed],
            ],
        )?;
        Ok(())
    }
}
