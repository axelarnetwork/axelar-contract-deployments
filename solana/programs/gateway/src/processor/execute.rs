use std::borrow::Cow;

use axelar_message_primitives::command::DecodedCommand;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::axelar_auth_weighted::OperatorshipTransferAllowed;
use crate::events::GatewayEvent;
use crate::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_execute(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let mut accounts_iter = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts_iter)?;
        let gateway_execute_data_pda = next_account_info(&mut accounts_iter)?;

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let mut gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        gateway_execute_data_pda.check_initialized_pda_without_deserialization(program_id)?;
        let execute_data =
            borsh::from_slice::<GatewayExecuteData>(&gateway_execute_data_pda.data.borrow())?;

        // Check: proof operators are known.
        let mut allow_operatorship_transfer = gateway_config
            .validate_proof(execute_data.command_batch_hash, execute_data.proof)
            .map_err(|err| {
                msg!("Proof validation failed: {:?}", err);
                ProgramError::InvalidArgument
            })?;

        if execute_data.command_batch.commands.len() != (accounts_iter.len()) {
            msg!("Mismatch between the number of commands and the number of accounts");
            return Err(ProgramError::InvalidArgument);
        }
        let commands = execute_data.command_batch.commands.into_iter();
        for (message_account, decoded_command) in accounts_iter.zip(commands) {
            // Check: The approved message PDA needs to already be initialized.
            let mut approved_command_account = {
                let approved_command_account = message_account
                    .as_ref()
                    .check_initialized_pda::<GatewayApprovedCommand>(program_id)?;

                // Check: Current message account represents the current approved command.
                let seed_hash = GatewayApprovedCommand::calculate_seed_hash(
                    gateway_root_pda.key,
                    &decoded_command,
                );
                approved_command_account.assert_valid_pda(&seed_hash, message_account.key);

                // https://github.com/axelarnetwork/cgp-spec/blob/c3010b9187ad9022dbba398525cf4ec35b75e7ae/solidity/contracts/AxelarGateway.sol#L103
                if approved_command_account.is_command_executed()
                    || approved_command_account.is_contract_call_approved()
                {
                    continue; // Ignore if duplicate commandId received
                }

                approved_command_account
            };

            match (decoded_command, &mut allow_operatorship_transfer) {
                (DecodedCommand::ApproveContractCall(decoded_command), _) => {
                    approved_command_account.set_ready_for_validate_contract_call()?;
                    let message_approved = decoded_command.into();
                    let event = GatewayEvent::MessageApproved(Cow::Borrowed(&message_approved));
                    event.emit()?;
                }
                (
                    DecodedCommand::TransferOperatorship(decoded_command),
                    allow_operatorship_transfer @ OperatorshipTransferAllowed::Allowed,
                ) => {
                    // Only 1 transfer allowed per batch
                    *allow_operatorship_transfer = OperatorshipTransferAllowed::NotAllowed;
                    approved_command_account.set_transfer_operatorship_executed()?;

                    // Perform the actual "transfer operatorship" process
                    GatewayEvent::OperatorshipTransferred(Cow::Borrowed(&decoded_command))
                        .emit()?;
                    // We just ignore all errors here, as we don't want to fail the entire batch
                    let _res = gateway_config.transfer_operatorship(decoded_command);

                    // Store the data back to the account.
                    let mut data = gateway_root_pda.try_borrow_mut_data()?;
                    gateway_config.pack_into_slice(&mut data);
                }
                (
                    DecodedCommand::TransferOperatorship(_),
                    OperatorshipTransferAllowed::NotAllowed,
                ) => {
                    // Rules for transfer operatorship:
                    // - Only 1 transfer allowed per batch, the rest are ignored
                    // - Transfer can only be done by the latest operator set
                    // And even then we don't want to fail the entire batch because the rest of the
                    // messages are valid so we just ignore the error and
                    // continue.
                    msg!("Operatorship transfer not allowed");
                    continue;
                }
            }

            // Save the updated approved message account
            let mut data = message_account.try_borrow_mut_data()?;
            approved_command_account.pack_into_slice(&mut data);
        }

        Ok(())
    }
}
