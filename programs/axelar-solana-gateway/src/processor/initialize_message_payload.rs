use crate::error::GatewayError;
use crate::state::incoming_message::IncomingMessage;
use crate::state::message_payload::MutMessagePayload;

use super::Processor;
use program_utils::{
    pda::{init_pda_raw, BytemuckedPda, ValidPDA},
    validate_system_account_key,
};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

impl Processor {
    /// Initializes a Program Derived Address (PDA) account to store message raw payload data.
    ///
    /// The account size is adjusted to accommodate both the payload contents and required metadata.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are missing.
    /// * Payer is not a signer or not writable.
    /// * Gateway root PDA is not initialized.
    /// * System program account is invalid.
    /// * Buffer size calculation overflows.
    /// * Account creation (sub instruction) fails.
    ///
    /// Returns [`GatewayError::MessagePayloadAlreadyInitialized`] if the message payload account is
    /// already initialized.
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * Converting the adjusted buffer size to usize overflows (via `expect`, unlikely to ever happen)
    pub fn process_initialize_message_payload(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        buffer_size: u64,
        command_id: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let incoming_message_account = next_account_info(accounts_iter)?;
        let message_payload_account = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        // Check: Payer is the signer
        if !payer.is_signer {
            solana_program::msg!("Error: payer account is not a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_writable {
            solana_program::msg!("Error: payer account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: Gateway root PDA
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        // Check: System Program
        validate_system_account_key(system_program.key)?;

        // Check: Message payload account is writable
        if !message_payload_account.is_writable {
            solana_program::msg!("Error: message payload account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        message_payload_account
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::MessagePayloadAlreadyInitialized)?;

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
        let (message_payload_pda, bump_seed) =
            crate::find_message_payload_pda(incoming_message_pda, *payer.key);
        if message_payload_account.key != &message_payload_pda {
            solana_program::msg!("Error: failed to derive message payload account address");
            return Err(ProgramError::InvalidArgument);
        }

        // Prepare the `create_account` instruction.
        let Ok(adjusted_account_size): Result<u64, _> = buffer_size
            .try_into()
            .map(MutMessagePayload::adjust_offset)
            .and_then(TryInto::try_into)
        else {
            solana_program::msg!("Failed to cast adjusted buffer size to u64");
            return Err(ProgramError::InvalidInstructionData);
        };

        let signers_seeds = &[
            crate::seed_prefixes::MESSAGE_PAYLOAD_SEED,
            incoming_message_pda.as_ref(),
            payer.key.as_ref(),
            &[bump_seed],
        ];

        init_pda_raw(
            payer,
            message_payload_account,
            program_id,
            system_program,
            adjusted_account_size,
            signers_seeds,
        )?;

        // Set the bump seed into account data
        let mut message_payload_account_data = message_payload_account.try_borrow_mut_data()?;
        let message_payload: MutMessagePayload<'_> = (*message_payload_account_data).try_into()?;
        *message_payload.bump = bump_seed;
        *message_payload.payload_hash = incoming_message.payload_hash;

        Ok(())
    }
}
