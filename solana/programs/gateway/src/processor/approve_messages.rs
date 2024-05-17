use std::borrow::Cow;

use axelar_message_primitives::command::DecodedCommand;
use itertools::*;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::events::GatewayEvent;
use crate::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};

impl Processor {
    /// Approves an array of messages, signed by the Axelar signers.
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/2eaf5199ee8ccc5eb1d8353c0dd7592feff0eb5c/contracts/gateway/AxelarAmplifierGateway.sol#L78-L84
    pub fn process_approve_messages(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        let mut accounts_iter = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts_iter)?;
        let gateway_appove_messages_execute_data_pda = next_account_info(&mut accounts_iter)?;

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let gateway_config = gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        gateway_appove_messages_execute_data_pda
            .check_initialized_pda_without_deserialization(program_id)?;
        let execute_data = borsh::from_slice::<GatewayExecuteData>(
            &gateway_appove_messages_execute_data_pda.data.borrow(),
        )?;

        // Check: proof operators are known.
        gateway_config
            .validate_proof(execute_data.command_batch_hash, &execute_data.proof)
            .map_err(|err| {
                msg!("Proof validation failed: {:?}", err);
                ProgramError::InvalidArgument
            })?;

        approve_messages(accounts_iter, execute_data, program_id, gateway_root_pda)?;

        Ok(())
    }
}

fn approve_messages<'a, 'b>(
    accounts_iter: impl Iterator<Item = &'a AccountInfo<'b>>,
    execute_data: GatewayExecuteData,
    program_id: &Pubkey,
    gateway_root_pda: &AccountInfo<'_>,
) -> Result<(), ProgramError>
where
    'b: 'a,
{
    for item in accounts_iter.zip_longest(execute_data.command_batch.commands.into_iter()) {
        let EitherOrBoth::Both(message_account, decoded_command) = item else {
            msg!("Mismatch between the number of commands and the number of accounts");
            return Err(ProgramError::InvalidArgument);
        };

        // Check: The approved message PDA needs to already be initialized.
        let Some(mut approved_command_account) = message_account
            .as_ref()
            .check_initialized_pda::<GatewayApprovedCommand>(program_id)?
            .command_valid_and_pending(gateway_root_pda.key, &decoded_command, message_account)?
        else {
            // https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/7902798e5fe62b3bc55935d2c1ee9c75aedd97cf/contracts/gateway/BaseAmplifierGateway.sol#L198-L200
            msg!("command not pending");
            continue;
        };

        let DecodedCommand::ApproveMessages(decoded_command) = decoded_command else {
            msg!(
                "Non-approve command provided to 'approve-messages': {:?}",
                decoded_command
            );
            return Err(ProgramError::InvalidArgument);
        };
        approved_command_account.set_ready_for_validate_contract_call()?;
        let message_approved = decoded_command.into();
        let event = GatewayEvent::MessageApproved(Cow::Borrowed(&message_approved));
        event.emit()?;

        // Save the updated approved message account
        let mut data = message_account.try_borrow_mut_data()?;
        approved_command_account.pack_into_slice(&mut data);
    }

    Ok(())
}
