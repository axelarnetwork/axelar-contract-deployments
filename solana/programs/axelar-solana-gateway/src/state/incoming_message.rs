//! Module for the IncomingMessage account type.

use std::mem::{self, size_of};

use bytemuck::{Pod, Zeroable};

/// Incoming message from Axelar -> Solana
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct IncomingMessageWrapper {
    /// The metadata for incoming message
    pub message: IncomingMessage,
    /// The bump that was used to create the PDA
    pub bump: u8,
    /// padding to align the bump
    pub _padding_bump: [u8; 7],
    /// padding to make struct size be 256 bytes
    pub _padding_size: [u8; 32],
    // .. the rest of the data on the PDA is the raw payload (not yet implemented)
}

/// Incoming message from Axelar -> Solana.
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct IncomingMessage {
    /// Length of the raw data
    pub data_len: u64, // 8 bytes
    /// Whilst writing the raw payload, this points to the beginning of next
    /// empty chunk
    pub data_pointer: u64, // 8 bytes
    /// Status of the message
    pub status: MessageStatus, // 4 byte
    /// alignment padding
    pub _padding: [u8; 4], // 4 bytes to align to 16 bytes
    /// Merkle root of the payload hash
    pub payload_hash: [u8; 32], // 32 bytes
}

impl IncomingMessage {
    /// New default [`IncomingMessage`]
    pub fn new(payload_hash: [u8; 32]) -> Self {
        Self {
            data_len: 0,
            // pad the pointer to the beginning of the next chunk to write the data into
            data_pointer: (size_of::<Self>() + size_of::<u8>())
                .try_into()
                .expect("valid u64"),
            status: MessageStatus::Approved,
            _padding: [0; 4],
            payload_hash,
        }
    }
}

impl IncomingMessageWrapper {
    /// Size, in bytes, to represent a value of this type.
    pub const LEN: usize = mem::size_of::<Self>();
}

/// After the command itself is marked as `Approved`, the command can be used
/// for CPI [`GatewayInstructon::ValidateMessage`] instruction.
/// This maps to [these lines in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L636-L648)
#[repr(C)]
#[derive(Zeroable, Copy, Clone, PartialEq, Eq, Debug)]
pub enum MessageStatus {
    /// [`GatewayInstructon::ValidateMessage`] has been called and the command
    /// has been executed by the destination program.
    Executed = 0,

    /// The state where the message has been approved and now its chunks are
    /// being written to
    InProgress = 1,

    /// The state of the command after it has been approved
    Approved = 2,
}

unsafe impl Pod for MessageStatus {}
