use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use super::event_utils::{read_array, read_string, EventParseError};
use super::Processor;
use crate::state::{BytemuckedPda, GatewayConfig};
use crate::{assert_valid_gateway_root_pda, event_prefixes};

impl Processor {
    /// This function is used to initialize the program.
    pub fn process_call_contract_offchain_data(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: String,
        destination_contract_address: String,
        payload_hash: [u8; 32],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = gateway_root_pda.try_borrow_data()?;
        let gateway_config = GatewayConfig::read(&data)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: sender is signer
        assert!(sender.is_signer, "Sender must be a signer");

        // Emit an event
        sol_log_data(&[
            event_prefixes::CALL_CONTRACT_OFFCHAIN_DATA,
            &sender.key.to_bytes(),
            &payload_hash,
            destination_chain.as_bytes(),
            destination_contract_address.as_bytes(),
        ]);
        Ok(())
    }
}

/// Represents a `CallContractEvent`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CallContractOffchainDataEvent {
    /// Sender's public key.
    pub sender_key: Pubkey,

    /// Payload hash, 32 bytes.
    pub payload_hash: [u8; 32],

    /// Destination chain as a `String`.
    pub destination_chain: String,

    /// Destination contract address as a `String`.
    pub destination_contract_address: String,
}

impl CallContractOffchainDataEvent {
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

        Ok(Self {
            sender_key,
            payload_hash,
            destination_chain,
            destination_contract_address,
        })
    }
}
