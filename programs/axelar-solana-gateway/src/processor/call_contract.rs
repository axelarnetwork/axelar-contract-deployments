use event_utils::{read_array, read_string, EventParseError};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::{
    assert_initialized_and_valid_gateway_root_pda, create_call_contract_signing_pda, event_prefixes,
};

impl Processor {
    /// This function initializes a cross-chain message by emitting an event containing the call details.
    ///
    /// The message can then be picked up by off-chain components for
    /// cross-chain delivery.
    ///
    /// It requires a valid signing PDA & signing PDA bump to be provided for verifying the
    /// authenticity of the call.
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
        _program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        destination_chain: &str,
        destination_contract_address: &str,
        payload: &[u8],
        signing_pda_bump: u8,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let sender = next_account_info(accounts_iter)?;
        let sender_signing_pda = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        assert_initialized_and_valid_gateway_root_pda(gateway_root_pda)?;

        if sender.is_signer {
            // Direct signer, so not a program, continue
        } else {
            // Case of a program, so a valid signing PDA must be provided
            let Ok(expected_signing_pda) =
                create_call_contract_signing_pda(*sender.key, signing_pda_bump)
            else {
                solana_program::msg!(
                    "Invalid call: sender must be a direct signer or a valid signing PDA must be provided",
                );
                return Err(GatewayError::CallerNotSigner.into());
            };

            if &expected_signing_pda != sender_signing_pda.key {
                // Signing PDA mismatch
                solana_program::msg!("Invalid call: a valid signing PDA must be provided",);
                return Err(GatewayError::InvalidSigningPDA.into());
            }

            if !sender_signing_pda.is_signer {
                // Signing PDA is correct but not a signer
                solana_program::msg!("Signing PDA must be a signer");
                return Err(GatewayError::CallerNotSigner.into());
            }

            // A valid signing PDA was provided and it's a signer, continue
        }

        // compute the payload hash
        let payload_hash = solana_program::keccak::hash(payload).to_bytes();

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
