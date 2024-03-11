use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::events::emit_message_approved_event;
use crate::get_gateway_root_config_pda;
use crate::state::{GatewayApprovedMessage, GatewayConfig, GatewayExecuteData};
use crate::types::execute_data_decoder::DecodedMessage;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_execute(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let mut accounts = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts)?;
        let execute_data_account = next_account_info(&mut accounts)?;
        let message_accounts: Vec<_> = accounts.collect();
        // Phase 1: Account validation

        // Check: Config account uses the canonical bump.
        let (canonical_pda, _canonical_bump) = get_gateway_root_config_pda();
        if *gateway_root_pda.key != canonical_pda {
            return Err(GatewayError::InvalidConfigAccount)?;
        }

        // Check: Config account is owned by the Gateway program.
        if *gateway_root_pda.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check: Config account is read only.
        if gateway_root_pda.is_writable {
            return Err(ProgramError::InvalidInstructionData);
        }

        // Check: execute_data account is owned by the Gateway program.
        if *execute_data_account.owner != crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // Check: execute_data account was initialized.
        let execute_data: GatewayExecuteData =
            borsh::from_slice(*execute_data_account.data.borrow())?;

        // Check: at least one message account.
        if message_accounts.is_empty() {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Phase 2: Deserialization & Proof Validation

        let Ok((proof, command_batch)) = execute_data.decode() else {
            return Err(GatewayError::MalformedProof)?;
        };

        // Check: proof is valid, internally.
        proof.validate(&command_batch.hash)?;

        // Unpack Gateway configuration data.
        let gateway_config: GatewayConfig = {
            let gateway_account_bytes = gateway_root_pda.try_borrow_mut_data()?;
            borsh::de::from_slice(&gateway_account_bytes)?
        };

        // Check: proof operators are known.
        gateway_config.validate_proof_operators(&proof.operators)?;

        // Phase 3: Update approved message accounts

        // Check approved message account initial premises post-validation so we iterate
        // on them only once.
        // TODO: Pairwise iterate over message accounts and validated commands from the
        // command batch.
        let mut last_visited_message_index = 0;
        for (&message_account, approved_command) in
            message_accounts.iter().zip(command_batch.commands.iter())
        {
            last_visited_message_index += 1;

            // Check: Current message account represents the current approved command.
            let (expected_pda, _bump, _hash) =
                GatewayApprovedMessage::pda(gateway_root_pda.key, &approved_command.into());
            if expected_pda != *message_account.key {
                return Err(ProgramError::InvalidSeeds);
            }

            // Check:: All messages must be "Approved".
            let mut approved_message =
                message_account.check_initialized_pda::<GatewayApprovedMessage>(program_id)?;

            if !approved_message.is_pending() {
                // TODO: use a more descriptive GatewayError variant here.
                return Err(ProgramError::AccountAlreadyInitialized);
            }

            // Success: update account message state to "Approved".
            // The message by default is in "approved" state.
            // https://github.com/axelarnetwork/axelar-cgp-solidity/blob/968c8964061f594c80dd111887edb93c5069e51e/contracts/AxelarGateway.sol#L509
            approved_message.set_approved();
            let mut data = message_account.try_borrow_mut_data()?;
            approved_message.pack_into_slice(&mut data);

            // Emit an event signaling message approval.
            let DecodedMessage {
                id,
                source_chain,
                source_address,
                destination_address,
                payload_hash,
                ..
            } = &approved_command.message;
            emit_message_approved_event(
                *id,
                source_chain.clone(),
                source_address.clone(),
                *destination_address,
                *payload_hash,
            )?;
        }

        // Check: all messages were visited
        if last_visited_message_index != message_accounts.len()
            || last_visited_message_index != command_batch.commands.len()
        {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        Ok(())
    }
}
