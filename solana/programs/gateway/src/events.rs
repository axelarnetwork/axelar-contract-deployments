//! Types used for logging messages.

use axelar_rkyv_encoding::types::{ArchivedMessage, Message};
use base64::engine::general_purpose;
use base64::Engine as _;
use rkyv::bytecheck::{self, CheckBytes};
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;
use crate::hasher_impl;

/// Logged when the Gateway receives an outbound message.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct CallContract {
    /// Message sender.
    pub sender: [u8; 32],
    /// The name of the target blockchain.
    pub destination_chain: String,
    /// The address of the target contract in the destination blockchain.
    pub destination_address: String,
    /// Contract call data.
    pub payload: Vec<u8>,
    /// The payload hash.
    pub payload_hash: [u8; 32],
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
/// Event that gets emitted when a message has been executed
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct MessageExecuted {
    /// The command id of the given message
    pub command_id: [u8; 32],
}

/// Emitted for every approved message after the Gateway validates a command
/// batch.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct MessageApproved {
    /// The command ID
    pub command_id: [u8; 32],
    /// Source chain.
    pub source_chain: Vec<u8>,
    /// The message id
    pub message_id: Vec<u8>,
    /// Source address.
    pub source_address: Vec<u8>,
    /// Destination address on Solana.
    pub destination_address: [u8; 32],
    /// The payload hash.
    pub payload_hash: [u8; 32],
}

/// Emitted when the latest signer set has been rotated
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct RotateSignersEvent {
    /// the new latet epoch
    pub new_epoch: crate::state::verifier_set_tracker::Epoch,
    /// the hash of the new signer set
    pub new_signers_hash: [u8; 32],
    /// little-endian encoded Pubkey that points to the ExecuteData PDA, which
    /// contains the full information about the latest signer set
    pub execute_data_pda: [u8; 32],
}

/// Event that gets emitted when the operatorship has been transferred
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct OperatorshipTransferred {
    /// little-endian encoded Pubkey for the latest operator
    pub operator: [u8; 32],
}

/// Gateway program logs.
///
/// Used internally by the Gateway program to log messages.
/// We are using Cow to avoid unnecessary allocations and NOT take
/// ownership of the data when emitting events.
#[non_exhaustive]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum GatewayEvent {
    /// Logged when the Gateway receives an outbound message.
    CallContract(CallContract),
    /// The event emitted after successful keys rotation.
    SignersRotated(RotateSignersEvent),
    /// Emitted for every approved message after the Gateway validates a command
    /// batch
    MessageApproved(MessageApproved),
    /// Emitted when a message has been executed
    MessageExecuted(MessageExecuted),
    /// Emitted when a the operatorship has been transferred
    OperatorshipTransferred(OperatorshipTransferred),
}

impl TryFrom<Message> for MessageApproved {
    type Error = GatewayError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let cc_id = message.cc_id();

        let destination_address: [u8; 32] = message
            .destination_address()
            .parse::<Pubkey>()
            .map(|pubkey| pubkey.to_bytes())
            .map_err(|_| GatewayError::PublicKeyParseError)?;

        Ok(MessageApproved {
            command_id: cc_id.command_id(hasher_impl()),
            message_id: cc_id.id().into(),
            source_chain: cc_id.chain().into(),
            source_address: message.source_address().into(),
            destination_address,
            payload_hash: message.payload_hash().to_owned(),
        })
    }
}

impl TryFrom<&ArchivedMessage> for MessageApproved {
    type Error = GatewayError;

    fn try_from(message: &ArchivedMessage) -> Result<Self, Self::Error> {
        let cc_id = message.cc_id();

        let destination_address: [u8; 32] = message
            .destination_address()
            .parse::<Pubkey>()
            .map(|pubkey| pubkey.to_bytes())
            .map_err(|_| GatewayError::PublicKeyParseError)?;

        Ok(MessageApproved {
            command_id: cc_id.command_id(hasher_impl()),
            message_id: cc_id.id().into(),
            source_chain: cc_id.chain().into(),
            source_address: message.source_address().into(),
            destination_address,
            payload_hash: message.payload_hash().to_owned(),
        })
    }
}

impl GatewayEvent {
    /// Emits the log for this event.
    pub fn emit(&self) -> Result<(), ProgramError> {
        let item = self.encode();
        sol_log_data(&[&item]);
        Ok(())
    }

    /// Encode the [`GatewayEvent`] into a [`Vec<u8>`] which satifies rkyv
    /// alignment requirements
    pub fn encode(&self) -> rkyv::AlignedVec {
        rkyv::to_bytes::<_, 0>(self).unwrap()
    }

    /// Try to parse a [`GatewayEvent`] out of a Solana program log line.
    pub fn parse_log<T: AsRef<str>>(log: T) -> Option<EventContainer> {
        let buffer = log
            .as_ref()
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64)
            .next()?;

        EventContainer::new(buffer)
    }
}

/// Wrapper around the rkyv encoded [`GatewayEvent`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContainer {
    buffer: Vec<u8>,
}

impl EventContainer {
    /// Create a new [`EventContainer`] from an rkyv enocded [`GatewayEvent`]
    ///
    /// The method will return `None` if the buffer cannod be deserialised into
    /// a valid [`ArchivedGatewayEvent`]
    pub fn new(buffer: Vec<u8>) -> Option<Self> {
        // check if this is a valid gateway event
        let _data = rkyv::check_archived_root::<GatewayEvent>(&buffer).ok()?;
        Some(Self { buffer })
    }

    /// Return a view into the buffer, deserialised
    pub fn parse(&self) -> &ArchivedGatewayEvent {
        // safe: we already checked that the buffer is valid when initializing it
        let data = unsafe { rkyv::archived_root::<GatewayEvent>(&self.buffer) };
        data
    }
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gateway_event_round_trip() {
        // Setup
        let call_contract = CallContract {
            sender: Pubkey::new_unique().to_bytes(),
            destination_chain: "ethereum".to_owned(),
            destination_address: "0x123...abc".to_owned(),
            payload: b"function_call()".to_vec(),
            payload_hash: [1; 32],
        };
        let rotate_signers_command = RotateSignersEvent {
            new_epoch: axelar_message_primitives::U256::from_u64(55),
            new_signers_hash: [42; 32],
            execute_data_pda: [55; 32],
        };
        let message_approved = MessageApproved {
            command_id: [2; 32],
            message_id: vec![2; 32],
            source_chain: b"solana".to_vec(),
            source_address: b"SourceAddress".to_vec(),
            destination_address: [3; 32],
            payload_hash: [4; 32],
        };
        let transfer_operatorship = OperatorshipTransferred {
            operator: [123; 32],
        };
        let message_executed = MessageExecuted {
            command_id: [255; 32],
        };
        let events = vec![
            GatewayEvent::CallContract(call_contract),
            GatewayEvent::SignersRotated(rotate_signers_command),
            GatewayEvent::MessageApproved(message_approved),
            GatewayEvent::OperatorshipTransferred(transfer_operatorship),
            GatewayEvent::MessageExecuted(message_executed),
        ];

        for event in events.into_iter() {
            // Action
            let event_encoded = event.encode();
            let event_encoded = general_purpose::STANDARD.encode(event_encoded.as_slice());
            let log = format!("Program log: {event_encoded}");

            let decoded_event_container = GatewayEvent::parse_log(log).unwrap();
            let decoded_event = decoded_event_container.parse();

            assert_eq!(
                &event, decoded_event,
                "pre-encoded and post-encoded events don't match"
            );
        }
    }
}
