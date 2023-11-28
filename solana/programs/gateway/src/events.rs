//! Types used for logging messages.

use borsh::BorshSerialize;
use solana_program::keccak;
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Logged when the Gateway receives an outbound message.
#[derive(BorshSerialize)]
pub struct ContractCallEvent<'a> {
    /// Message sender.
    pub sender: &'a Pubkey,
    /// The name of the target blockchain.
    pub destination_chain: &'a str,
    /// The address of the target contract in the destination blockchain.
    pub destination_contract_address: &'a str,
    /// The payload hash.
    pub payload_hash: [u8; 32],
    /// Contract call data.
    pub payload: &'a [u8],
}

impl<'a> ContractCallEvent<'a> {
    /// Constructs a new `ContractCallEvent`.
    pub fn new(
        sender: &'a Pubkey,
        destination_chain: &'a str,
        destination_contract_address: &'a str,
        payload: &'a [u8],
    ) -> Self {
        Self {
            sender,
            destination_chain,
            destination_contract_address,
            payload_hash: keccak::hash(payload).to_bytes(),
            payload,
        }
    }
}

/// Emits a `ContractCallEvent`.
pub fn emit_call_contract_event(
    sender: &Pubkey,
    destination_chain: &str,
    destination_contract_address: &str,
    payload: &[u8],
) {
    let event = ContractCallEvent::new(
        sender,
        destination_chain,
        destination_contract_address,
        payload,
    );

    // TODO: match previous implementation layout.
    let bytes = &[
        &event.sender.as_ref(),
        event.destination_chain.as_bytes(),
        event.destination_contract_address.as_bytes(),
        &event.payload_hash,
        &event.payload_hash,
    ];
    sol_log_data(bytes);
}
