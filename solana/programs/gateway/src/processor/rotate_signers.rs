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
        let gateway_approve_messages_execute_data_pda = next_account_info(&mut accounts_iter)?;
        let message_account = next_account_info(&mut accounts_iter)?;
        let operator = next_account_info(&mut accounts_iter);

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let mut gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        gateway_approve_messages_execute_data_pda
            .check_initialized_pda_without_deserialization(program_id)?;
        let execute_data = borsh::from_slice::<GatewayExecuteData>(
            &gateway_approve_messages_execute_data_pda.data.borrow(),
        )?;

        let [decoded_command @ DecodedCommand::RotateSigners(rotate_signers_command)] =
            execute_data.command_batch.commands.as_slice()
        else {
            msg!("expected exactly one `RotateSigners` command");
            return Err(ProgramError::InvalidArgument);
        };

        // we always enforce the delay unless unless the operator has been provided and
        // its also the Gateway opreator
        // refence: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/c290c7337fd447ecbb7426e52ac381175e33f602/contracts/gateway/AxelarAmplifierGateway.sol#L98-L101
        let enforce_rotation_delay = operator.map_or(true, |operator| {
            let operator_matches = *operator.key == gateway_config.operator;
            let operator_is_sigener = operator.is_signer;
            // if the operator matches and is also the signer - disable rotation delay
            !(operator_matches && operator_is_sigener)
        });

        let mut approved_command_account = message_account
            .as_ref()
            .check_initialized_pda::<GatewayApprovedCommand>(program_id)?
            .command_valid_and_pending(gateway_root_pda.key, decoded_command, message_account)?
            .ok_or_else(|| {
                msg!("Command already executed");
                ProgramError::InvalidArgument
            })?;

        // Check: proof signer set is known.
        let signer_data = gateway_config
            .validate_proof(execute_data.command_batch_hash, &execute_data.proof)
            .map_err(|err| {
                msg!("Proof validation failed: {:?}", err);
                ProgramError::InvalidArgument
            })?;

        // Check: proof is signed by latest signers
        if enforce_rotation_delay && !matches!(signer_data, SignerSetMetadata::Latest) {
            msg!("Proof is not signed by the latest signer set");
            return Err(ProgramError::InvalidArgument);
        }

        // Set command state as executed
        approved_command_account.set_signers_rotated_executed()?;

        // Save the updated approved message account
        let mut data = message_account.try_borrow_mut_data()?;
        approved_command_account.pack_into_slice(&mut data);

        // Try to set the new signer set - but if we fail, it's not an error because we
        // still need to persist the command execution state.
        // If rotate_signers_command is a repeat signer set, this will revert
        if let Err(err) = gateway_config.rotate_signers(rotate_signers_command) {
            msg!("Failed to rotate signers {:?}", err);
            return Ok(());
        };

        // Emit event if the signers were rotated
        GatewayEvent::SignersRotated(Cow::Borrowed(rotate_signers_command)).emit()?;

        // Store the gateway data back to the account.
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        gateway_config.pack_into_slice(&mut data);

        Ok(())
    }
}
