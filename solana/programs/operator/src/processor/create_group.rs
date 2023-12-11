//! Create a new group

use program_utils::{check_program_account, init_pda};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::hash::hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::{assert_operator_account, assert_operator_group_account, Processor};
use crate::check_id;
use crate::state::{OperatorAccount, OperatorGroupAccount};

impl Processor {
    /// Create a new group
    pub fn process_create_group(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        id: String,
    ) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let operator_group_account = next_account_info(account_info_iter)?;
        let operator_account = next_account_info(account_info_iter)?;
        let operator_account_owner = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        let bump_seed =
            assert_operator_group_account(operator_group_account, program_id, id.as_str())?;
        if *operator_group_account.owner != system_program::id() {
            return Err(ProgramError::IllegalOwner);
        }

        let id_h = hash(id.as_bytes());
        init_pda(
            funder_info,
            operator_group_account,
            program_id,
            system_program_info,
            OperatorGroupAccount::new(id_h.to_bytes()),
            &[&id_h.to_bytes(), &[bump_seed]],
        )?;

        let bump_seed = assert_operator_account(
            operator_account,
            operator_group_account,
            operator_account_owner,
            program_id,
        )?;
        if *operator_account_owner.owner != system_program::id() {
            return Err(ProgramError::IllegalOwner);
        }
        init_pda(
            funder_info,
            operator_account,
            program_id,
            system_program_info,
            OperatorAccount::new_active(),
            &[
                &operator_account_owner.key.to_bytes(),
                &operator_group_account.key.as_ref(),
                &[bump_seed],
            ],
        )?;

        Ok(())
    }
}
