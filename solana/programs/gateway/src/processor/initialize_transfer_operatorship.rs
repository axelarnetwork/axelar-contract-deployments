use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::error::GatewayError;
use crate::state::transfer_operatorship::TransferOperatorshipAccount;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_transfer_operatorship(
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        operators_and_weights: Vec<(crate::types::address::Address, crate::types::u256::U256)>,
        threshold: crate::types::u256::U256,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let transfer_operatorship_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Transfer operatorship account uses the canonical bump.
        let transfer_operatorship =
            TransferOperatorshipAccount::new(operators_and_weights, threshold);
        let (expected_pda, bump, seeds) = transfer_operatorship.pda_with_seeds();

        if *transfer_operatorship_account.key != expected_pda {
            return Err(GatewayError::InvalidAccountAddress.into());
        }

        super::init_pda(
            payer,
            transfer_operatorship_account,
            &[seeds.as_ref(), &[bump]],
            &transfer_operatorship,
        )
    }
}
