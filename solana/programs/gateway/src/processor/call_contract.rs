use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::events::emit_call_contract_event;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_call_contract(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        destination_chain: Vec<u8>,
        destination_contract_address: Vec<u8>,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        // TODO we want to deserialize the PDA as well, otherwise we can't check if it's
        // initialized
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        let payload_hash = solana_program::keccak::hash(&payload).to_bytes();

        assert!(sender.is_signer, "Sender must be a signer");

        emit_call_contract_event(
            *sender.key,
            destination_chain,
            destination_contract_address,
            payload,
            payload_hash,
        )?;
        Ok(())
    }
}
