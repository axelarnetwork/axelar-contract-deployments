//! Add an operator to an operator group

use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::{check_program_account, init_pda};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::check_id;
use crate::processor::assert_operator_account;
use crate::state::{OperatorAccount, OperatorGroupAccount};

impl Processor {
    /// Add an operator to an operator group
    pub fn process_add_operator(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        check_program_account(program_id, check_id)?;

        let account_info_iter = &mut accounts.iter();

        let funder_info = next_account_info(account_info_iter)?;
        let existing_operator_group_pda_account = next_account_info(account_info_iter)?;
        let existing_operator_pda_account = next_account_info(account_info_iter)?;
        let existing_operator_owner_account = next_account_info(account_info_iter)?;
        let new_operator_owner_account = next_account_info(account_info_iter)?;
        let new_operator_pda_account = next_account_info(account_info_iter)?;
        let system_program_info = next_account_info(account_info_iter)?;

        if existing_operator_group_pda_account.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }
        let data = OperatorGroupAccount::try_from_slice(
            &existing_operator_group_pda_account.data.borrow(),
        )?;
        assert!(
            data.is_initialized(),
            "OperatorGroupAccount is not initialized"
        );
        assert!(
            existing_operator_owner_account.is_signer,
            "Existing operator account must be signer"
        );
        // Make sure existing operator account is associated with the operator group
        // account
        let _ = assert_operator_account(
            existing_operator_pda_account,
            existing_operator_group_pda_account,
            existing_operator_owner_account,
            program_id,
        )?;
        // Create a new operator account
        let bump_seed = assert_operator_account(
            new_operator_pda_account,
            existing_operator_group_pda_account,
            new_operator_owner_account,
            program_id,
        )?;

        if new_operator_pda_account.owner == &system_program::id() {
            // Create a new PDA
            init_pda(
                funder_info,
                new_operator_pda_account,
                program_id,
                system_program_info,
                OperatorAccount::new_active(),
                &[
                    &new_operator_owner_account.key.to_bytes(),
                    &existing_operator_group_pda_account.key.as_ref(),
                    &[bump_seed],
                ],
            )?;
        } else if *new_operator_pda_account.owner == *program_id {
            // Override existing account
            let mut account_data = new_operator_pda_account.try_borrow_mut_data()?;
            let mut data = OperatorAccount::try_from_slice(&account_data[..account_data.len()])?;
            data.make_active();
            let serialized_data = data.try_to_vec()?;
            account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
        } else {
            return Err(ProgramError::IllegalOwner);
        }

        Ok(())
    }
}
