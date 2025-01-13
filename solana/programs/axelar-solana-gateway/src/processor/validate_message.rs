use core::str::FromStr;

use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_encoding::LeafHash;
use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::log::sol_log_data;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::event_utils::{read_array, read_string, EventParseError};
use super::Processor;
use crate::error::GatewayError;
use crate::state::incoming_message::{command_id, IncomingMessage, MessageStatus};
use crate::{
    assert_valid_incoming_message_pda, create_validate_message_signing_pda, event_prefixes,
};

impl Processor {
    /// Validate a message approval, and mark it as used
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Account balance and expected ownership validation fails.
    /// * Required accounts are missing.
    ///
    /// Returns [`GatewayError`] if:
    /// * `Message` not in approved state.
    /// * `Message` hash does not match with `IncomingMessage`'s.
    /// * Invalid destination address format.
    /// * Caller PDA validation fails.
    /// * Signing authority missing.
    /// * Data serialization fails.
    pub fn process_validate_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message: &Message,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let caller = next_account_info(accounts_iter)?;

        // compute the message hash
        let message_hash = message.hash::<SolanaSyscallHasher>();

        // compute the command id
        let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);

        // Check: Gateway Root PDA is initialized.
        incoming_message_pda.check_initialized_pda_without_deserialization(program_id)?;
        let mut data = incoming_message_pda.try_borrow_mut_data()?;
        let incoming_message =
            IncomingMessage::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_incoming_message_pda(
            &command_id,
            incoming_message.bump,
            incoming_message_pda.key,
        )?;

        // Check: message is approved
        if !incoming_message.status.is_approved() {
            return Err(GatewayError::MessageNotApproved.into());
        }
        // Check: message hashes match
        if incoming_message.message_hash != message_hash {
            return Err(GatewayError::MessageHasBeenTamperedWith.into());
        }
        let destination_address = Pubkey::from_str(&message.destination_address)
            .map_err(|_err| GatewayError::InvalidDestinationAddress)?;

        // check that caller ir valid signing PDA
        let expected_signing_pda = create_validate_message_signing_pda(
            &destination_address,
            incoming_message.signing_pda_bump,
            &command_id,
        )?;
        if &expected_signing_pda != caller.key {
            msg!("Invalid signing PDA");
            return Err(GatewayError::InvalidSigningPDA.into());
        }
        // check that caller is signer
        if !caller.is_signer {
            return Err(GatewayError::CallerNotSigner.into());
        }

        incoming_message.status = MessageStatus::executed();

        // Emit an event
        sol_log_data(&[
            event_prefixes::MESSAGE_EXECUTED,
            &command_id,
            &destination_address.to_bytes(),
            &message.payload_hash,
            message.cc_id.chain.as_bytes(),
            message.cc_id.id.as_bytes(),
            message.source_address.as_bytes(),
            message.destination_chain.as_bytes(),
        ]);

        Ok(())
    }
}

/// Represents a message event.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MessageEvent {
    /// Command identifier, 32 bytes.
    pub command_id: [u8; 32],

    /// Destination address as a `Pubkey`.
    pub destination_address: Pubkey,

    /// Payload hash, 32 bytes.
    pub payload_hash: [u8; 32],

    /// CC ID chain as a `String`.
    pub cc_id_chain: String,

    /// CC ID ID as a `String`.
    pub cc_id_id: String,

    /// Source address as a `String`.
    pub source_address: String,

    /// Destination chain as a `String`.
    pub destination_chain: String,
}

impl MessageEvent {
    /// Constructs a new `MessageEvent` by parsing the provided data iterator.
    ///
    /// # Arguments
    ///
    /// * `data` - An iterator over `Vec<u8>` slices representing the event data.
    ///
    /// # Errors
    ///
    /// Returns a `EventParseError` if any required data is missing or invalid.
    pub fn new<I: Iterator<Item = Vec<u8>>>(mut data: I) -> Result<Self, EventParseError> {
        // Read known-size elements
        let command_id_data = data
            .next()
            .ok_or(EventParseError::MissingData("command_id"))?;
        let command_id = read_array::<32>("command_id", &command_id_data)?;

        let destination_address_data = data
            .next()
            .ok_or(EventParseError::MissingData("destination_address"))?;
        let destination_address = Pubkey::new_from_array(read_array::<32>(
            "destination_address",
            &destination_address_data,
        )?);

        let payload_hash_data = data
            .next()
            .ok_or(EventParseError::MissingData("payload_hash"))?;
        let payload_hash = read_array::<32>("payload_hash", &payload_hash_data)?;

        // Read dynamic-size elements
        let cc_id_chain_data = data
            .next()
            .ok_or(EventParseError::MissingData("cc_id_chain"))?;
        let cc_id_chain = read_string("cc_id_chain", cc_id_chain_data)?;

        let cc_id_id_data = data
            .next()
            .ok_or(EventParseError::MissingData("cc_id_id"))?;
        let cc_id_id = read_string("cc_id_id", cc_id_id_data)?;

        let source_address_data = data
            .next()
            .ok_or(EventParseError::MissingData("source_address"))?;
        let source_address = read_string("source_address", source_address_data)?;

        let destination_chain_data = data
            .next()
            .ok_or(EventParseError::MissingData("destination_chain"))?;
        let destination_chain = read_string("destination_chain", destination_chain_data)?;

        Ok(Self {
            command_id,
            destination_address,
            payload_hash,
            cc_id_chain,
            cc_id_id,
            source_address,
            destination_chain,
        })
    }
}
