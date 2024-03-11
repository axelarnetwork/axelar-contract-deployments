use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::events::emit_operatorship_transferred_event;
use crate::state::transfer_operatorship::TransferOperatorshipAccount;
use crate::state::GatewayConfig;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_transfer_operatorship(
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> Result<(), ProgramError> {
        // Extract required accounts.
        let accounts_iter = &mut accounts.iter();
        let payer_account = next_account_info(accounts_iter)?;
        let new_operators_account = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: Config account is the canonical PDA.
        let (expected_pda_info, _bump) = crate::get_gateway_root_config_pda();
        super::helper::compare_address(gateway_root_pda, expected_pda_info)?;

        // Check: Config account is owned by the Gateway program.
        if *gateway_root_pda.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check: New operators account is owned by the Gateway program.
        if *new_operators_account.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Unpack the data from the new operators account.
        let new_operators_bytes: &[u8] = &new_operators_account.data.borrow();
        let new_operators =
            borsh::de::from_slice::<TransferOperatorshipAccount>(new_operators_bytes)?;

        // Check: New operators account is the expected PDA.
        let (expected_new_operators_pda, _bump) = new_operators.pda();
        super::helper::compare_address(new_operators_account, expected_new_operators_pda)?;

        // Check: new operator data is valid.
        new_operators.validate().map_err(GatewayError::from)?;

        // Hash the new operator set.
        let new_operators_hash = new_operators.hash();

        // Unpack Gateway configuration data.
        let mut config: GatewayConfig = {
            let state_bytes_ref = gateway_root_pda.try_borrow_mut_data()?;
            borsh::de::from_slice(&state_bytes_ref)?
        };

        // Update epoch and operators.
        config
            .operators_and_epochs
            .update(new_operators_hash)
            .map_err(GatewayError::from)?;

        // Resize and refund state account space.
        config.reallocate(gateway_root_pda, payer_account, system_account)?;

        // Write the packed data back to the state account.
        let serialized_state = borsh::to_vec(&config)?;
        let mut state_data_ref = gateway_root_pda.try_borrow_mut_data()?;
        state_data_ref[..serialized_state.len()].copy_from_slice(&serialized_state);

        // Emit an event to signal the successful operatorship transfer
        emit_operatorship_transferred_event(*new_operators_account.key)?;
        Ok(())
    }
}
