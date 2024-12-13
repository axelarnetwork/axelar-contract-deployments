use crate::state::message_payload::MessagePayload;

use super::Processor;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

impl Processor {
    /// Closes a message payload PDA and reclaim its lamports.
    pub fn process_close_message_payload(
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

        // Parse the message payload from the account data
        let mut account_data = message_payload_account.try_borrow_mut_data()?;
        let message_payload = MessagePayload::from_borrowed_account_data(&mut account_data)?;

        // Check: Buffer PDA can be derived from provided seeds.
        let message_payload_pda = crate::create_message_payload_pda(
            *gateway_root_pda.key,
            command_id,
            *payer.key,
            *message_payload.bump,
        )?;
        assert_eq!(&message_payload_pda, message_payload_account.key,);

        // Close the Buffer PDA account and reclaim its lamports
        **payer.try_borrow_mut_lamports()? += message_payload_account.lamports();
        **message_payload_account.try_borrow_mut_lamports()? = 0;

        Ok(())
    }
}
