use super::Processor;
use crate::assert_initialized_and_valid_gateway_root_pda;
use crate::state::incoming_message::IncomingMessage;
use crate::state::message_payload::MutMessagePayload;
use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

impl Processor {
    /// Commits a message payload PDA by computing a hash of its contents and storing it in the
    /// account state.
    ///
    /// Once committed, the payload becomes immutable and can be safely referenced.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are missing or in wrong order
    /// * Payer is not a signer
    /// * Gateway root PDA or message payload account is not initialized
    /// * Message payload PDA derivation fails or address mismatch
    pub fn process_commit_message_payload(
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
        assert_initialized_and_valid_gateway_root_pda(gateway_root_pda)?;

        // Check: Message Payload account is initialized
        message_payload_account.check_initialized_pda_without_deserialization(&crate::ID)?;

        // Parse the message payload account from the account data.
        let mut message_payload_account_data = message_payload_account.try_borrow_mut_data()?;
        let mut message_payload: MutMessagePayload<'_> =
            (*message_payload_account_data).try_into()?;

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

        // Check: Message Payload PDA can be derived from provided seeds.
        let incoming_message_pda = *incoming_message_account.key;
        crate::assert_valid_message_payload_pda(
            incoming_message_pda,
            *payer.key,
            *message_payload.bump,
            message_payload_account.key,
        )?;

        // Finally, calculate the hash check that it matches the incoming message hash.
        let payload_hash = message_payload.hash_raw_payload_bytes();
        if &payload_hash.to_bytes() != message_payload.payload_hash {
            return Err(ProgramError::InvalidAccountData);
        }

        // Commit the message payload, which also check that the message was not previously committed.
        message_payload.commit()?;

        Ok(())
    }
}
