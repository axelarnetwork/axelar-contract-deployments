mod address;
mod axelar_message;
pub mod command;
mod payload;

pub use address::*;
pub use axelar_message::*;
pub use payload::*;

// TODO: Optimisation - try using bytemuck crate
/// This is the payload that the `execute` processor on the destinatoin program
/// must expect
#[derive(Debug, PartialEq, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
#[repr(C)]
pub struct AxelarExecutablePayload {
    /// The command_id which is the unique identifier for the Axelar command
    ///
    /// The Axelar Message CCID, truncated to 32 bytes during proof
    /// generation.
    pub command_id: [u8; 32],
    /// The payload *without* the prefixed accounts
    ///
    /// This needs to be done by the relayer before calling the destination
    /// program
    pub payload_without_accounts: Vec<u8>,
    /// The source chain of the command
    pub source_chain: Vec<u8>,
    /// The source address of the command
    pub source_address: Vec<u8>,
}

// TODO: Optimisation - try using bytemuck crate
/// This is the wrapper instruction that the destination program should expect
/// as the incoming &[u8]
#[derive(Debug, PartialEq, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub enum AxelarCallableInstruction<T> {
    /// The payload is coming from the Axelar Gateway (submitted by the relayer)
    AxelarExecute(AxelarExecutablePayload),
    /// The payload is coming from the user
    Native(T),
}
