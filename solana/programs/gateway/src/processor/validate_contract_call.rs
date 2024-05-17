use axelar_message_primitives::command::{ApproveMessagesCommand, DecodedCommand};
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::state::{GatewayApprovedCommand, GatewayConfig};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_validate_contract_call(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        command: ApproveMessagesCommand,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let approved_message_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let caller = next_account_info(accounts_iter)?;

        let mut approved_message =
            approved_message_pda.check_initialized_pda::<GatewayApprovedCommand>(program_id)?;
        let _gateway_root_pda =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        let command_id = command.command_id;
        let command = DecodedCommand::ApproveMessages(command);
        let seed_hash = GatewayApprovedCommand::calculate_seed_hash(gateway_root_pda.key, &command);

        // Check: we only operate on approved contract call command types
        let DecodedCommand::ApproveMessages(command) = command else {
            msg!("Invalid command type after we had just constr");
            return Err(ProgramError::InvalidArgument);
        };
        // Check: the seed hash is correct for the given PDA
        approved_message.assert_valid_pda(&seed_hash, approved_message_pda.key);

        // Action
        approved_message.validate_contract_call(
            &command_id,
            &command.destination_program,
            caller,
        )?;

        // Store the data back to the account.
        let mut data = approved_message_pda.try_borrow_mut_data()?;
        approved_message.pack_into_slice(&mut data);

        Ok(())
    }
}
