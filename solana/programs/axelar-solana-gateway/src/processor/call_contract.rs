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
    /// This function initializes a cross-chain message by emitting an event containing the call details.
    ///
    /// The message can then be picked up by off-chain components for
    /// cross-chain delivery.
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
    /// Emits a `CALL_CONTRACT` event with the following data:
    /// * Sender's public key
    /// * Keccak256 hash of the payload
    /// * Destination chain identifier
    /// * Destination contract address
    /// * Raw payload data
    pub fn process_call_contract(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: &str,
        destination_contract_address: &str,
        payload: &[u8],
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

        // compute the payload hash
        let payload_hash = solana_program::keccak::hash(payload).to_bytes();

        // Check: sender is signer
        if !sender.is_signer {
            solana_program::msg!("Error: Sender must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Emit an event
        sol_log_data(&[
            event_prefixes::CALL_CONTRACT,
            &sender.key.to_bytes(),
            &payload_hash,
            destination_chain.as_bytes(),
            destination_contract_address.as_bytes(),
            payload,
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
    /// Constructs a new `CallContractEvent` by parsing event data from an iterator.
    ///
    /// # Data Format Details
    ///
    /// This method parses a sequence of byte vectors representing a cross-chain contract call event,
    /// expecting data in the following order:
    ///
    /// - `sender_key`: 32-byte Solana public key
    /// - `payload_hash`: 32-byte hash of the payload
    /// - `destination_chain`: UTF-8 string identifying target blockchain
    /// - `destination_contract_address`: UTF-8 string of target contract
    /// - `payload`: Raw bytes to be transmitted cross-chain
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
    ///
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
