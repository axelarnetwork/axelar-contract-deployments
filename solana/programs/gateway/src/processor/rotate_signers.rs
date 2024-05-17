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
use crate::axelar_auth_weighted::SignerSetMetadata;
use crate::events::GatewayEvent;
use crate::state::{GatewayApprovedCommand, GatewayConfig, GatewayExecuteData};

impl Processor {
    /// Rotate the weighted signers, signed off by the latest Axelar signers.
    /// The minimum rotation delay is enforced by default, unless the caller is
    /// the gateway operator.
    ///
    /// The gateway operator allows recovery in case of an incorrect/malicious
    /// rotation, while still requiring a valid proof from a recent signer set.
    ///
    /// Rotation to duplicate signers is rejected.
    ///
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/9dae93af0b799e536005951ddc36284132813579/contracts/gateway/AxelarAmplifierGateway.sol#L94
    pub fn process_rotate_signers(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        let mut accounts_iter = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts_iter)?;
        let gateway_appove_messages_execute_data_pda = next_account_info(&mut accounts_iter)?;
        let message_account = next_account_info(&mut accounts_iter)?;

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let mut gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        gateway_appove_messages_execute_data_pda
            .check_initialized_pda_without_deserialization(program_id)?;
        let execute_data = borsh::from_slice::<GatewayExecuteData>(
            &gateway_appove_messages_execute_data_pda.data.borrow(),
        )?;

        let [decoded_command @ DecodedCommand::RotateSigners(transfer_ops)] =
            execute_data.command_batch.commands.as_slice()
        else {
            msg!("expected exactly one `RotateSigners` command");
            return Err(ProgramError::InvalidArgument);
        };

        // todo: check if we need to eforce rotation delay

        let mut approved_command_account = message_account
            .as_ref()
            .check_initialized_pda::<GatewayApprovedCommand>(program_id)?
            .command_valid_and_pending(gateway_root_pda.key, decoded_command, message_account)?
            .ok_or_else(|| {
                msg!("Command already execited");
                ProgramError::InvalidArgument
            })?;

        // Check: proof operators are known.
        let signer_data = gateway_config
            .validate_proof(execute_data.command_batch_hash, &execute_data.proof)
            .map_err(|err| {
                msg!("Proof validation failed: {:?}", err);
                ProgramError::InvalidArgument
            })?;

        // Check: proof is signed by latest signers
        let SignerSetMetadata::Latest = signer_data else {
            msg!("Proof is not signed by the latest signer set");
            return Err(ProgramError::InvalidArgument);
        };

        // Set command state as executed
        approved_command_account.set_transfer_operatorship_executed()?;

        // Save the updated approved message account
        let mut data = message_account.try_borrow_mut_data()?;
        approved_command_account.pack_into_slice(&mut data);

        // Try to set the new signer set - but if we fail, it's not an error because we
        // still need to persist the command execution state.
        if let Err(err) = gateway_config.rotate_signers(transfer_ops) {
            msg!("Failed to rotate signers {:?}", err);
            return Ok(());
        };

        // Emit event if the operatorship was transferred.
        GatewayEvent::OperatorshipTransferred(Cow::Borrowed(transfer_ops)).emit()?;

        // Store the gatewau data back to the account.
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        gateway_config.pack_into_slice(&mut data);

        Ok(())
    }
}
