use std::borrow::Cow;

use axelar_message_primitives::{
    AxelarMessageParams, CommandId, DataPayloadHash, DestinationProgramId, SourceAddress,
    SourceChain,
};
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::Processor;
use crate::error::GatewayError;
use crate::state::GatewayApprovedMessage;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_initialize_approved_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message_id: [u8; 32],
        source_chain: String,
        source_address: Vec<u8>,
        payload_hash: [u8; 32],
        destination_program: DestinationProgramId,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let approved_message_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // TODO validate gateway root pda

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Approved message account uses the canonical bump.
        let (canonical_pda, bump, seeds) = GatewayApprovedMessage::pda(
            gateway_root_pda.key,
            &AxelarMessageParams {
                command_id: CommandId(Cow::Owned(message_id)),
                source_chain: SourceChain(Cow::Owned(source_chain)),
                source_address: SourceAddress(&source_address),
                destination_program,
                payload_hash: DataPayloadHash(Cow::Borrowed(&payload_hash)),
            },
        );

        approved_message_pda.check_uninitialized_pda()?;
        if *approved_message_pda.key != canonical_pda {
            return Err(GatewayError::InvalidApprovedMessageAccount.into());
        }

        program_utils::init_pda(
            payer,
            approved_message_pda,
            program_id,
            system_account,
            GatewayApprovedMessage::pending(bump),
            &[seeds.as_ref(), &[bump]],
        )
    }
}
