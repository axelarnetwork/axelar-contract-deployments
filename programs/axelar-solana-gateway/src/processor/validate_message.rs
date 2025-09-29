use core::str::FromStr;

use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_encoding::LeafHash;
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::events::MessageExecutedEvent;
use crate::state::incoming_message::{command_id, IncomingMessage, MessageStatus};
use crate::{
    assert_initialized_and_valid_gateway_root_pda, assert_valid_incoming_message_pda,
    create_validate_message_signing_pda,
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
        let gateway_root_pda = next_account_info(accounts_iter)?;
        event_cpi_accounts!(accounts_iter);

        // Check: Gateway Root PDA is initialized.
        assert_initialized_and_valid_gateway_root_pda(gateway_root_pda)?;

        // compute the message hash
        let message_hash = message.hash::<SolanaSyscallHasher>();

        // compute the command id
        let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);

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

        // check that caller is valid signing PDA
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

        emit_cpi!(MessageExecutedEvent {
            command_id,
            destination_address,
            payload_hash: message.payload_hash,
            source_chain: message.cc_id.chain.clone(),
            cc_id: message.cc_id.id.clone(),
            source_address: message.source_address.clone(),
            destination_chain: message.destination_chain.clone(),
        });

        Ok(())
    }
}
