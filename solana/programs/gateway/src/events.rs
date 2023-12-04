//! Types used for logging messages.

use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

/// Gateway program logs.
#[repr(u8)]
#[derive(Debug)]
pub enum GatewayEvent<'a> {
    /// Logged when the Gateway receives an outbound message.
    CallContract {
        /// Message sender.
        sender: &'a Pubkey,
        /// The name of the target blockchain.
        destination_chain: &'a [u8],
        /// The address of the target contract in the destination blockchain.
        destination_address: &'a [u8],
        /// Contract call data.
        payload: &'a [u8],
        /// The payload hash.
        payload_hash: &'a [u8; 32],
    },
}

impl<'a> GatewayEvent<'a> {
    /// Returns the event's discriminant byte.
    fn discriminant(&self) -> u8 {
        unsafe { *(self as *const Self as *const u8) }
    }
    /// Emits the log for this event.
    pub fn emit(&self) {
        match *self {
            GatewayEvent::CallContract {
                sender,
                destination_chain,
                destination_address,
                payload_hash,
                payload,
            } => sol_log_data(&[
                &[self.discriminant()],
                sender.as_ref(),
                destination_chain,
                destination_address,
                payload,
                payload_hash,
            ]),
        };
    }

    /// Try to parse a [`GatewayEvent`] out of a log line.
    pub fn parse_log(_log: &str) -> Option<Self> {
        todo!("implement this")
    }
}

/// Emits a `ContractCallEvent`.
pub fn emit_call_contract_event(
    sender: &Pubkey,
    destination_chain: &[u8],
    destination_contract_address: &[u8],
    payload: &[u8],
    payload_hash: &[u8; 32],
) -> Result<(), GatewayError> {
    let event = GatewayEvent::CallContract {
        sender,
        destination_chain,
        destination_address: destination_contract_address,
        payload_hash,
        payload,
    };
    event.emit();
    Ok(())
}
