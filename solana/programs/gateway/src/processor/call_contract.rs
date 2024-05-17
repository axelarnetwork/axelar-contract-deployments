use std::borrow::Cow;

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::events::{CallContract, GatewayEvent};
use crate::state::GatewayConfig;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_call_contract(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: Vec<u8>,
        destination_contract_address: Vec<u8>,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let _ = gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        let payload_hash = solana_program::keccak::hash(&payload).to_bytes();

        assert!(sender.is_signer, "Sender must be a signer");

        let call_contract = CallContract {
            destination_chain,
            payload,
            payload_hash,
            sender: *sender.key,
            destination_address: destination_contract_address,
        };
        let event = GatewayEvent::CallContract(Cow::Borrowed(&call_contract));
        event.emit()?;
        Ok(())
    }
}
