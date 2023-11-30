//! Types used for logging messages.

use std::array::TryFromSliceError;

use solana_program::keccak;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

/// Logged when the Gateway receives an outbound message.
pub struct ContractCallEventRef<'a> {
    /// Message sender.
    pub sender: Pubkey,
    /// The name of the target blockchain.
    pub destination_chain: &'a [u8],
    /// The address of the target contract in the destination blockchain.
    pub destination_contract_address: &'a [u8],
    /// The payload hash.
    pub payload_hash: &'a [u8],
    /// Contract call data.
    pub payload: &'a [u8],
}

impl<'a> ContractCallEventRef<'a> {
    /// Constructs a new `ContractCallEvent`.
    pub fn new(
        sender: Pubkey,
        destination_chain: &'a [u8],
        destination_contract_address: &'a [u8],
        payload_hash: &'a [u8],
        payload: &'a [u8],
    ) -> Result<Self, GatewayError> {
        if payload_hash.len() != 32 {
            return Err(GatewayError::InvalidMessagePayloadHash);
        }

        Ok(Self {
            sender,
            destination_chain,
            destination_contract_address,
            payload_hash,
            payload,
        })
    }

    /// Copy values into a [`ContractCallEvent`].
    /// Returns an error if the payload hash slice don't fit into a `[u8; 32]`.
    pub fn try_to_owned(&self) -> Result<ContractCallEvent, TryFromSliceError> {
        Ok(ContractCallEvent {
            sender: self.sender.to_owned(),
            destination_chain: self.destination_chain.to_vec(),
            destination_contract_address: self.destination_contract_address.to_vec(),
            payload_hash: self.payload_hash.try_into()?,
            payload: self.payload.to_vec(),
        })
    }
}

/// Owned version of [`ContractCallEventRef`]
pub struct ContractCallEvent {
    /// Message sender.
    pub sender: Pubkey,
    /// The name of the target blockchain.
    pub destination_chain: Vec<u8>,
    /// The address of the target contract in the destination blockchain.
    pub destination_contract_address: Vec<u8>,
    /// The payload hash.
    pub payload_hash: [u8; 32],
    /// Contract call data.
    pub payload: Vec<u8>,
}

impl<'a> ContractCallEvent {
    /// Returns a [`ContractCallEventRef`].
    pub fn borrow(&'a self) -> ContractCallEventRef<'a> {
        ContractCallEventRef {
            sender: self.sender,
            destination_chain: &self.destination_chain,
            destination_contract_address: &self.destination_contract_address,
            payload_hash: &self.payload_hash,
            payload: &self.payload,
        }
    }
}

/// Emits a `ContractCallEvent`.
pub fn emit_call_contract_event(
    sender: Pubkey,
    destination_chain: &[u8],
    destination_contract_address: &[u8],
    payload: &[u8],
) -> Result<(), GatewayError> {
    let payload_hash = keccak::hash(payload).to_bytes();

    let event = ContractCallEventRef::new(
        sender,
        destination_chain,
        destination_contract_address,
        &payload_hash,
        payload,
    )
    .map_err(|_| GatewayError::InvalidMessagePayloadHash)?;

    // TODO: match previous implementation layout.
    let bytes: &[&[u8]] = &[
        &event.sender.as_ref(),
        &event.destination_chain,
        &event.destination_contract_address,
        &event.payload,
        &event.payload_hash,
    ];
    sol_log_data(bytes);
    Ok(())
}
