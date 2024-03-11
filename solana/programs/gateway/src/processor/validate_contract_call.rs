use std::borrow::Cow;

use axelar_message_primitives::{
    AxelarMessageParams, CommandId, DataPayloadHash, DestinationProgramId, SourceAddress,
    SourceChain,
};
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::state::GatewayApprovedMessage;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_validate_contract_call(
        program_id: &Pubkey,
        command_id: [u8; 32],
        source_chain: String,
        source_address: Vec<u8>,
        payload_hash: [u8; 32],
        destination_pubkey: DestinationProgramId,
        accounts: &[AccountInfo],
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let approved_message_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let caller = next_account_info(accounts_iter)?;

        let mut approved_message =
            approved_message_pda.check_initialized_pda::<GatewayApprovedMessage>(program_id)?;

        let message_id = CommandId(Cow::Owned(command_id));
        let seed_hash = GatewayApprovedMessage::calculate_seed_hash(
            gateway_root_pda.key,
            &AxelarMessageParams {
                command_id: message_id.clone(),
                source_chain: SourceChain(Cow::Owned(source_chain)),
                source_address: SourceAddress(&source_address),
                destination_program: destination_pubkey,
                payload_hash: DataPayloadHash(Cow::Borrowed(&payload_hash)),
            },
        );
        let approved_message_derived_pda = Pubkey::create_program_address(
            &[seed_hash.as_ref(), &[approved_message.bump]],
            program_id,
        )?;
        if *approved_message_pda.key != approved_message_derived_pda {
            return Err(GatewayError::InvalidAccountAddress.into());
        }

        // Action
        approved_message.verify_caller(&message_id, &destination_pubkey, caller)?;

        // Store the data back to the account.
        let mut data = approved_message_pda.try_borrow_mut_data()?;
        approved_message.pack_into_slice(&mut data);

        Ok(())
    }
}
