use super::Processor;
use crate::state::message_payload::MutMessagePayload;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

impl Processor {
    /// Commits a message payload PDA by hashing its contents and persisting the resulting
    /// hash into account state.
    pub fn process_commit_message_payload(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        command_id: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let message_payload_account = next_account_info(accounts_iter)?;

        // Check: Payer is the signer
        assert!(payer.is_signer);

        // Check: Gateway root PDA
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        // Check: Message Payload account is initialized
        message_payload_account.check_initialized_pda_without_deserialization(&crate::ID)?;

        // Parse the message payload account from the account data.
        let mut message_payload_account_data = message_payload_account.try_borrow_mut_data()?;
        let mut message_payload: MutMessagePayload<'_> =
            (*message_payload_account_data).try_into()?;

        // Check: Message Payload PDA can be derived from provided seeds.
        let message_payload_pda = crate::create_message_payload_pda(
            *gateway_root_pda.key,
            command_id,
            *payer.key,
            *message_payload.bump,
        )?;
        assert_eq!(message_payload_account.key, &message_payload_pda);

        // Check: Message payload PDA must not be committed.
        message_payload.assert_uncommitted()?;

        // Finally, calculate the hash and commit it.
        message_payload.hash_raw_payload_bytes();
        Ok(())
    }
}
