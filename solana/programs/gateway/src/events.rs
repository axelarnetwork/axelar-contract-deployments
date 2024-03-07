//! Types used for logging messages.
use base64::engine::general_purpose;
use base64::Engine as _;
use borsh::{self, BorshDeserialize, BorshSerialize};
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::types::PubkeyWrapper;

/// Gateway program logs.
///
/// Used internally by the Gateway program to log messages.
// TODO: We should use the `std::borrow::Cow` to avoid unecessary allocations.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum GatewayEvent {
    /// Logged when the Gateway receives an outbound message.
    CallContract {
        /// Message sender.
        sender: PubkeyWrapper,
        /// The name of the target blockchain.
        destination_chain: Vec<u8>,
        /// The address of the target contract in the destination blockchain.
        destination_address: Vec<u8>,
        /// Contract call data.
        payload: Vec<u8>,
        /// The payload hash.
        payload_hash: [u8; 32],
    },
    /// The event emited after successful keys rotation.
    OperatorshipTransferred {
        /// Pubkey of the account that stores the key rotation information.
        info_account_address: PubkeyWrapper,
    },
    /// Emitted for every approved message after the Gateway validates a command
    /// batch.
    MessageApproved {
        /// The Message ID
        message_id: [u8; 32],
        /// Source chain.
        source_chain: String,
        /// Source address.
        source_address: String,
        /// Destination address on Solana.
        destination_address: [u8; 32],
        /// The payload hash.
        payload_hash: [u8; 32],
    },
}

impl GatewayEvent {
    /// Emits the log for this event.
    pub fn emit(&self) -> Result<(), ProgramError> {
        let serialized = borsh::to_vec(self)?;
        sol_log_data(&[&serialized]);
        Ok(())
    }

    /// Try to parse a [`GatewayEvent`] out of a Solana program log line.
    pub fn parse_log<T: AsRef<str>>(log: T) -> Option<Self> {
        let cleaned_input = log
            .as_ref()
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64)
            .next()?;
        borsh::from_slice(&cleaned_input).ok()
    }
}

/// Emits a [`ContractCallEventRef`].
pub fn emit_call_contract_event(
    sender: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    payload_hash: [u8; 32],
) -> Result<(), ProgramError> {
    let event = GatewayEvent::CallContract {
        sender,
        destination_chain,
        destination_address,
        payload,
        payload_hash,
    };
    event.emit()
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

/// Emit a [`OperatorshipTransferred`].
pub fn emit_operatorship_transferred_event(pubkey: Pubkey) -> Result<(), ProgramError> {
    let event = GatewayEvent::OperatorshipTransferred {
        info_account_address: pubkey,
    };
    event.emit()
}

/// Emit a [`MessageApproved`].
pub fn emit_message_approved_event(
    id: [u8; 32],
    source_chain: String,
    source_address: String,
    destination_address: [u8; 32],
    payload_hash: [u8; 32],
) -> Result<(), ProgramError> {
    let event = GatewayEvent::MessageApproved {
        message_id: id,
        source_chain,
        source_address,
        destination_address,
        payload_hash,
    };
    event.emit()
}
