use std::str::FromStr;

use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_encoding::LeafHash;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::events::{GatewayEvent, MessageExecuted};
use crate::state::incoming_message::{command_id, IncomingMessageWrapper, MessageStatus};
use crate::{assert_valid_incoming_message_pda, create_validate_message_signing_pda};

impl Processor {
    /// Validate a message approval, and mark it as used
    pub fn process_validate_message(
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message: Message,
        signing_pda_bump: u8,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let caller = next_account_info(accounts_iter)?;

        let message_hash = message.hash::<SolanaSyscallHasher>();
        let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);

        // check that message is approved
        let mut data = incoming_message_pda.try_borrow_mut_data()?;
        let data_bytes: &mut [u8; IncomingMessageWrapper::LEN] =
            (*data).try_into().map_err(|_err| {
                solana_program::msg!("incoming message account data is corrupt");
                ProgramError::InvalidAccountData
            })?;
        let incoming_message = bytemuck::cast_mut::<_, IncomingMessageWrapper>(data_bytes);
        if incoming_message.message.status != MessageStatus::Approved {
            msg!("message not approved");
            return Err(ProgramError::InvalidAccountData);
        }
        if incoming_message.message.message_hash != message_hash {
            msg!("message has been tampered with");
            return Err(ProgramError::InvalidInstructionData);
        }
        assert_valid_incoming_message_pda(
            &command_id,
            incoming_message.bump,
            incoming_message_pda.key,
        )?;
        let destination_address =
            Pubkey::from_str(&message.destination_address).map_err(|_err| {
                msg!("Destination address is not a valid Pubkey");
                ProgramError::InvalidArgument
            })?;

        // check that caller ir valid signing PDA
        let expected_signing_pda = create_validate_message_signing_pda(
            &destination_address,
            signing_pda_bump,
            &command_id,
        )?;
        if &expected_signing_pda != caller.key {
            msg!("Invalid signing PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        // check that caller is signer
        if !caller.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        incoming_message.message.status = MessageStatus::Executed;

        // Emit an event
        GatewayEvent::MessageExecuted(MessageExecuted {
            command_id,
            source_chain: message.cc_id.chain,
            message_id: message.cc_id.id,
            source_address: message.source_address,
            destination_address: message.destination_address,
            payload_hash: message.payload_hash,
        })
        .emit()?;

        Ok(())
    }
}
