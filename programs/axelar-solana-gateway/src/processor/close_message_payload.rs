use crate::state::incoming_message::IncomingMessage;
use crate::state::message_payload::MutMessagePayload;

use super::Processor;
use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

impl Processor {
    /// Closes a message payload PDA account and reclaims its lamports back to the payer.
    ///
    /// Typically used after a message has been fully processed or when cleaning up unused message
    /// accounts.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are missing or in wrong order
    /// * Payer is not a signer
    /// * Gateway root PDA is not properly initialized
    /// * Message payload account is not properly initialized
    /// * Message payload PDA derivation fails
    /// * Message payload account address doesn't match derived address
    pub fn process_close_message_payload(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        command_id: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let incoming_message_account = next_account_info(accounts_iter)?;
        let message_payload_account = next_account_info(accounts_iter)?;

        // Check: payer is signer
        if !payer.is_signer {
            solana_program::msg!("Error: payer must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Check: Gateway root PDA
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        // Check: Message Payload account is initialized
        message_payload_account.check_initialized_pda_without_deserialization(&crate::ID)?;

        // Parse the message payload from the account data
        let mut account_data = message_payload_account.try_borrow_mut_data()?;
        let message_payload: MutMessagePayload<'_> = (*account_data).try_into()?;

        // Check: Incoming Message PDA account is initialized and validate it
        incoming_message_account.check_initialized_pda_without_deserialization(program_id)?;
        let incoming_message_data = incoming_message_account.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data).ok_or_else(|| {
            solana_program::msg!("Error: failed to read incoming message account data");
            ProgramError::InvalidAccountData
        })?;

        // Validate the IncomingMessage PDA using the stored bump
        crate::assert_valid_incoming_message_pda(
            &command_id,
            incoming_message.bump,
            incoming_message_account.key,
        )?;

        // Check: Buffer PDA can be derived from provided seeds.
        let incoming_message_pda = *incoming_message_account.key;
        let message_payload_pda = crate::create_message_payload_pda(
            incoming_message_pda,
            *payer.key,
            *message_payload.bump,
        )?;
        if &message_payload_pda != message_payload_account.key {
            solana_program::msg!("Error: failed to derive message payload account address");
            return Err(ProgramError::InvalidSeeds);
        }

        // Close the Buffer PDA account
        program_utils::pda::close_pda(payer, message_payload_account, &crate::ID)?;

        Ok(())
    }
}
