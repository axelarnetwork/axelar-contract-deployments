use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::event_utils::{read_array, read_string, EventParseError};
use super::Processor;
use crate::error::GatewayError;
use crate::state::GatewayConfig;
use crate::{assert_valid_gateway_root_pda, event_prefixes};

impl Processor {
    /// Processes a cross-chain contract call using off-chain data and emits the appropriate event.
    ///
    /// This function is similar to [`Processor::process_call_contract`] but accepts a pre-computed
    /// payload hash instead of the raw payload bytes.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Required accounts are not provided
    /// * Gateway root PDA is not properly initialized
    /// * Gateway root PDA's bump seed is invalid
    /// * Sender is not a signer
    ///
    /// Returns [`GatewayError`] if:
    /// * Gateway configuration data is invalid (`BytemuckDataLenInvalid`)
    ///
    /// # Events
    ///
    /// Emits a `CALL_CONTRACT_OFFCHAIN_DATA` event with the following data:
    /// * Sender's public key
    /// * Pre-computed payload hash
    /// * Destination chain identifier
    /// * Destination contract address
    pub fn process_call_contract_offchain_data(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: &str,
        destination_contract_address: &str,
        payload_hash: [u8; 32],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let data = gateway_root_pda.try_borrow_data()?;
        let gateway_config =
            GatewayConfig::read(&data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: sender is signer
        if !sender.is_signer {
            solana_program::msg!("Error: Sender must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

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
    /// Constructs a new `CallContractOffchainDataEvent` by parsing event data from an iterator.
    ///
    /// # Data Format Details
    ///
    /// - `sender_key`: 32-byte Solana public key of the transaction signer
    /// - `payload_hash`: 32-byte hash referencing off-chain payload data
    /// - `destination_chain`: UTF-8 string identifying target blockchain
    /// - `destination_contract_address`: UTF-8 string of target contract
    ///
    /// # Errors
    ///
    /// Returns [`EventParseError::MissingData`] if:
    /// * Any required field is missing from the iterator
    /// * Field name is included in the error message
    ///
    /// Returns [`EventParseError::InvalidUtf8`] if:
    /// * Strings are not valid UTF-8
    ///
    /// Returns [`EventParseError::InvalidLength`] if:
    /// * Public key or payload hash are not exactly 32 bytes
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
