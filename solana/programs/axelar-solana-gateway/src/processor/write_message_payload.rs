use super::Processor;
use crate::state::message_payload::MutMessagePayload;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

impl Processor {
    /// Writes bytes to a message payload PDA at a specified offset.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Account balance and expected ownership validation fails.
    /// * `MessagePayload` PDA derivation fails .
    /// * Data borrowing fails.
    ///
    /// Returns custom error if:
    /// * Payer is not a signer.
    /// * `MessagePayload` account  is already committed.
    /// * Write operation exceeds bounds.
    /// * Data serialization fails.
    pub fn process_write_message_payload(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        offset: usize,
        bytes_to_write: &[u8],
        command_id: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let message_payload_account = next_account_info(accounts_iter)?;

        // Check: Payer is the signer
        if !payer.is_signer {
            solana_program::msg!("Error: payer account is not a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Check: Gateway root PDA
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        // Check: Message Payload account is initialized
        message_payload_account.check_initialized_pda_without_deserialization(&crate::ID)?;

        // Parse the message payload account from the account data.
        let mut account_data = message_payload_account.try_borrow_mut_data()?;
        let mut message_payload: MutMessagePayload<'_> = (*account_data).try_into()?;

        // Check: Message Payload PDA can be derived from provided seeds.
        let message_payload_pda = crate::create_message_payload_pda(
            *gateway_root_pda.key,
            command_id,
            *payer.key,
            *message_payload.bump,
        )?;
        if message_payload_account.key != &message_payload_pda {
            solana_program::msg!("Error: failed to derive message payload account address");
            return Err(ProgramError::InvalidArgument);
        }

        // Check: Message payload PDA must not be committed
        message_payload.assert_uncommitted()?;

        // Write the bytes
        message_payload.write(bytes_to_write, offset)
    }
}
