use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use super::event_utils::{read_array, read_string, EventParseError};
use super::Processor;
use crate::event_prefixes;
use crate::state::GatewayConfig;

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_call_contract(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: String,
        destination_contract_address: String,
        payload: Vec<u8>,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let _ = gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        let payload_hash = solana_program::keccak::hash(&payload).to_bytes();

        assert!(sender.is_signer, "Sender must be a signer");

        // Emit an event
        sol_log_data(&[
            event_prefixes::CALL_CONTRACT,
            &sender.key.to_bytes(),
            &payload_hash,
            destination_chain.as_bytes(),
            destination_contract_address.as_bytes(),
            payload.as_slice(),
        ]);
        Ok(())
    }
}

/// Represents a `CallContractEvent`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallContractEvent {
    /// Sender's public key.
    pub sender_key: Pubkey,

    /// Payload hash, 32 bytes.
    pub payload_hash: [u8; 32],

    /// Destination chain as a `String`.
    pub destination_chain: String,

    /// Destination contract address as a `String`.
    pub destination_contract_address: String,

    /// Payload data as a `Vec<u8>`.
    pub payload: Vec<u8>,
}

impl CallContractEvent {
    /// Constructs a new `CallContractEvent` by parsing the provided data iterator.
    pub fn new<I>(mut data: I) -> Result<Self, EventParseError>
    where
        I: Iterator<Item = Vec<u8>>,
    {
        let sender_key_data = data
            .next()
            .ok_or(EventParseError::MissingData("sender_key"))?;
        let sender_key = read_array::<32>("sender_key", &sender_key_data)?;
        let sender_key = Pubkey::new_from_array(sender_key);

        let payload_hash_data = data
            .next()
            .ok_or(EventParseError::MissingData("payload_hash"))?;
        let payload_hash = read_array::<32>("payload_hash", &payload_hash_data)?;

        let destination_chain_data = data
            .next()
            .ok_or(EventParseError::MissingData("destination_chain"))?;
        let destination_chain = read_string("destination_chain", destination_chain_data)?;

        let destination_contract_address_data = data
            .next()
            .ok_or(EventParseError::MissingData("destination_contract_address"))?;
        let destination_contract_address = read_string(
            "destination_contract_address",
            destination_contract_address_data,
        )?;

        let payload_data = data.next().ok_or(EventParseError::MissingData("payload"))?;

        Ok(Self {
            sender_key,
            payload_hash,
            destination_chain,
            destination_contract_address,
            payload: payload_data,
        })
    }
}
