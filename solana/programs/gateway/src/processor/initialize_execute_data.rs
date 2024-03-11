use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::error::GatewayError;
use crate::state::GatewayExecuteData;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_execute_data(
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        execute_data: &GatewayExecuteData,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let _gateway_root_pda = next_account_info(accounts_iter)?;
        let execute_data_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Execute Data account uses the canonical bump.
        let (canonical_pda, bump, seeds) = execute_data.pda();
        if *execute_data_account.key != canonical_pda {
            return Err(GatewayError::InvalidExecuteDataAccount.into());
        }
        super::init_pda(
            payer,
            execute_data_account,
            &[seeds.as_ref(), &[bump]],
            execute_data,
        )
    }
}
