use std::borrow::Cow;

use itertools::*;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::commands::ArchivedCommand;
use crate::events::GatewayEvent;
use crate::state::verifier_set_tracker::VerifierSetTracker;
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
        let gateway_approve_messages_execute_data_pda = next_account_info(&mut accounts_iter)?;
        let verifier_set_tracker = next_account_info(&mut accounts_iter)?;

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let gateway_config = gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // unpack verifier set tracker
        let verifier_set_tracker =
            match verifier_set_tracker.check_initialized_pda::<VerifierSetTracker>(program_id) {
                Ok(set) => set,
                Err(err) => {
                    msg!("Invalid VerifierSetTracker PDA");
                    return Err(err);
                }
            };

        // unpack execute data
        gateway_approve_messages_execute_data_pda
            .check_initialized_pda_without_deserialization(program_id)?;
        let borrowed_account_data = gateway_approve_messages_execute_data_pda.data.borrow();
        let execute_data = GatewayExecuteData::new(
            *borrowed_account_data,
            gateway_root_pda.key,
            &gateway_config.domain_separator,
        )?;

        // Check: proof operators are known.
        gateway_config
            .validate_proof(
                execute_data.payload_hash,
                execute_data.proof(),
                &verifier_set_tracker,
            )
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
    execute_data: GatewayExecuteData<'_>,
    program_id: &Pubkey,
    gateway_root_pda: &AccountInfo<'_>,
) -> Result<(), ProgramError>
where
    'b: 'a,
{
    let Some(messages) = execute_data.messages() else {
        msg!("Non-approve command provided to 'approve-messages'");
        return Err(ProgramError::InvalidArgument);
    };

    for item in accounts_iter.zip_longest(messages.iter()) {
        let EitherOrBoth::Both(message_account, axelar_message) = item else {
            msg!("Mismatch between the number of commands and the number of accounts");
            return Err(ProgramError::InvalidArgument);
        };

        let command = ArchivedCommand::from(axelar_message);
        // Check: The approved message PDA needs to already be initialized.
        let Some(mut approved_command_account) = message_account
            .as_ref()
            .check_initialized_pda::<GatewayApprovedCommand>(program_id)?
            .command_valid_and_pending(gateway_root_pda.key, &command, message_account)?
        else {
            // https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/7902798e5fe62b3bc55935d2c1ee9c75aedd97cf/contracts/gateway/BaseAmplifierGateway.sol#L198-L200
            msg!("command not pending");
            continue;
        };

        approved_command_account.set_ready_for_validate_message()?;
        let message_approved = axelar_message.try_into()?;
        let event = GatewayEvent::MessageApproved(Cow::Borrowed(&message_approved));
        event.emit()?;

        // Save the updated approved message account
        let mut data = message_account.try_borrow_mut_data()?;
        approved_command_account.pack_into_slice(&mut data);
    }

    Ok(())
}
