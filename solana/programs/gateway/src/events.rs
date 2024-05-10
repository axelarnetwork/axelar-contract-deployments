//! Types used for logging messages.

use std::borrow::Cow;

use axelar_message_primitives::command::{
    ApproveContractCallCommand, DecodedCommand, TransferOperatorshipCommand,
};
use base64::engine::general_purpose;
use base64::Engine as _;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
/// Logged when the Gateway receives an outbound message.
pub struct CallContract {
    /// Message sender.
    pub sender: Pubkey,
    /// The name of the target blockchain.
    pub destination_chain: Vec<u8>,
    /// The address of the target contract in the destination blockchain.
    pub destination_address: Vec<u8>,
    /// Contract call data.
    pub payload: Vec<u8>,
    /// The payload hash.
    pub payload_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, BorshDeserialize, BorshSerialize)]
/// Emitted for every approved message after the Gateway validates a command
/// batch.
pub struct MessageApproved {
    /// The Message ID
    pub message_id: [u8; 32],
    /// Source chain.
    pub source_chain: Vec<u8>,
    /// Source address.
    pub source_address: Vec<u8>,
    /// Destination address on Solana.
    pub destination_address: [u8; 32],
    /// The payload hash.
    pub payload_hash: [u8; 32],
}

/// Gateway program logs.
///
/// Used internally by the Gateway program to log messages.
/// We are using Cow to avoid unnecessary allocations and NOT take
/// ownership of the data when emitting events.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, PartialEq, Clone, BorshSerialize)]
pub enum GatewayEvent<'a> {
    /// Logged when the Gateway receives an outbound message.
    CallContract(Cow<'a, CallContract>),
    /// The event emited after successful keys rotation.
    OperatorshipTransferred(Cow<'a, TransferOperatorshipCommand>),
    /// Emitted for every approved message after the Gateway validates a command
    /// batch
    MessageApproved(Cow<'a, MessageApproved>),
}

// Custom deserialization implementation for `GatewayEvent`.
// Reason: Borsh does not support deserializing data that has lifetime bounds,
// so we need to handle it ourselves.
impl<'a> BorshDeserialize for GatewayEvent<'a> {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let tag = u8::deserialize_reader(reader)?;
        match tag {
            0 => Ok(GatewayEvent::CallContract(Cow::Owned(
                CallContract::deserialize_reader(reader)?,
            ))),
            1 => Ok(GatewayEvent::OperatorshipTransferred(Cow::Owned(
                TransferOperatorshipCommand::deserialize_reader(reader)?,
            ))),
            2 => Ok(GatewayEvent::MessageApproved(Cow::Owned(
                MessageApproved::deserialize_reader(reader)?,
            ))),
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid tag: {}", tag),
            )),
        }
    }
}

impl From<DecodedCommand> for GatewayEvent<'_> {
    fn from(command: DecodedCommand) -> Self {
        match command {
            DecodedCommand::ApproveContractCall(appove_call_command) => {
                GatewayEvent::MessageApproved(Cow::Owned(appove_call_command.into()))
            }
            DecodedCommand::TransferOperatorship(transfer_operatorship_command) => {
                GatewayEvent::OperatorshipTransferred(Cow::Owned(transfer_operatorship_command))
            }
        }
    }
}

impl From<ApproveContractCallCommand> for MessageApproved {
    fn from(command: ApproveContractCallCommand) -> Self {
        MessageApproved {
            message_id: command.command_id,
            source_chain: command.source_chain,
            source_address: command.source_address,
            destination_address: command.destination_program.0.to_bytes(),
            payload_hash: command.payload_hash,
        }
    }
}

impl<'a> GatewayEvent<'a> {
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
            sender: Pubkey::new_unique(),
            destination_chain: b"ethereum".to_vec(),
            destination_address: b"0x123...abc".to_vec(),
            payload: b"function_call()".to_vec(),
            payload_hash: [1; 32],
        };
        let transfer_operatorship_command = TransferOperatorshipCommand {
            command_id: [1; 32],
            destination_chain: 222,
            operators: vec![],
            weights: vec![],
            quorum: 42,
        };
        let message_approved = MessageApproved {
            message_id: [2; 32],
            source_chain: b"solana".to_vec(),
            source_address: b"SourceAddress".to_vec(),
            destination_address: [3; 32],
            payload_hash: [4; 32],
        };
        let events_owned = vec![
            GatewayEvent::CallContract(Cow::Owned(call_contract.clone())),
            GatewayEvent::OperatorshipTransferred(Cow::Owned(
                transfer_operatorship_command.clone(),
            )),
            GatewayEvent::MessageApproved(Cow::Owned(message_approved.clone())),
        ];
        let events_borrowed = vec![
            GatewayEvent::CallContract(Cow::Borrowed(&call_contract)),
            GatewayEvent::OperatorshipTransferred(Cow::Borrowed(&transfer_operatorship_command)),
            GatewayEvent::MessageApproved(Cow::Borrowed(&message_approved)),
        ];

        for (event_owned, event_borrowed) in
            events_owned.into_iter().zip(events_borrowed.into_iter())
        {
            // Action
            let serialized_borrowed = borsh::to_vec(&event_borrowed).unwrap();
            let deserialized_borrowed = borsh::from_slice(&serialized_borrowed).unwrap();
            let serialized_owned = borsh::to_vec(&event_owned).unwrap();
            let deserialized_owned = borsh::from_slice(&serialized_owned).unwrap();

            // Assert - every combination should be equal
            assert_eq!(event_owned, deserialized_borrowed);
            assert_eq!(event_borrowed, deserialized_borrowed);
            assert_eq!(event_owned, deserialized_owned);
            assert_eq!(event_borrowed, deserialized_owned);
        }
    }
}
