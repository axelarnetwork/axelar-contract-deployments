use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use super::{Processor, ToBytes};
use crate::error::GatewayError;
use crate::seed_prefixes;
use crate::state::execute_data::ExecuteDataVariant;
use crate::state::{GatewayConfig, GatewayExecuteData};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_execute_data<T>(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        execute_data: Vec<u8>,
    ) -> Result<(), ProgramError>
    where
        GatewayExecuteData<T>: ToBytes,
        T: ExecuteDataVariant,
    {
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

        let Ok(execute_data) =
            GatewayExecuteData::<T>::new(&execute_data, gateway_root_pda.key, &domain_separator)
        else {
            msg!("Failed to deserialize execute_data bytes");
            return Err(ProgramError::InvalidAccountData);
        };

        // Check: Execute Data account is not initialized.
        if let Err(err) = execute_data_account.check_uninitialized_pda() {
            msg!("Execute Datat PDA already initialized");
            return Err(err);
        }
        // Check: Execute Data PDA is correctly derived
        crate::assert_valid_execute_data_pda(
            &execute_data,
            gateway_root_pda.key,
            execute_data_account.key,
        );

        super::init_pda_with_dynamic_size(
            payer,
            execute_data_account,
            &[
                seed_prefixes::EXECUTE_DATA_SEED,
                gateway_root_pda.key.as_ref(),
                &execute_data.hash_decoded_contents(),
                &[execute_data.bump],
            ],
            &execute_data,
        )
    }
}
