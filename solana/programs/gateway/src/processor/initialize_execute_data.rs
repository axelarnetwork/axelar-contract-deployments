use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::error::GatewayError;
use crate::state::{GatewayConfig, GatewayExecuteData};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_execute_data(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        execute_data: Vec<u8>,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let execute_data_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Gateway Root PDA is initialized.
        let domain_separator = gateway_root_pda
            .check_initialized_pda::<GatewayConfig>(program_id)?
            .domain_separator;

        let execute_data =
            GatewayExecuteData::new(&execute_data, gateway_root_pda.key, &domain_separator)?;

        // Check: Execute Data account is not initialized.
        execute_data_account.check_uninitialized_pda()?;
        // Check: Execute Data PDA is correctly derived
        execute_data.assert_valid_pda(gateway_root_pda.key, execute_data_account.key);

        // Check: Execute Data account uses the canonical bump.
        let (canonical_pda, bump, seeds) = execute_data.pda(gateway_root_pda.key);
        if *execute_data_account.key != canonical_pda {
            return Err(GatewayError::InvalidExecuteDataAccount.into());
        }
        super::init_pda_with_dynamic_size(
            payer,
            execute_data_account,
            &[seeds.as_ref(), &[bump]],
            &execute_data,
        )
    }
}
