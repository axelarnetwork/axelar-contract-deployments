//! Module for the IncomingMessage account type.

use bytemuck::{Pod, Zeroable};

use super::BytemuckedPda;

/// Data for the incoming message (from Axelar to Solana) PDA.
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct IncomingMessage {
    /// The bump that was used to create the PDA
    pub bump: u8,
    /// The bump for the signing PDA
    pub signing_pda_bump: u8,
    _pad: [u8; 2],
    /// Status of the message
    pub status: MessageStatus, // 4 byte
    /// Hash of the whole message
    pub message_hash: [u8; 32],
    /// Hash of the message's payload
    pub payload_hash: [u8; 32],
}

impl IncomingMessage {
    /// New default [`IncomingMessage`].
    pub fn new(
        bump: u8,
        signing_pda_bump: u8,
        status: MessageStatus,
        message_hash: [u8; 32],
        payload_hash: [u8; 32],
    ) -> Self {
        Self {
            bump,
            signing_pda_bump,
            _pad: Default::default(),
            status,
            message_hash,
            payload_hash,
        }
    }

    /// Size of this type, in bytes.
    pub const LEN: usize = std::mem::size_of::<Self>();
}

impl BytemuckedPda for IncomingMessage {}

/// If this is marked as `Approved`, the command can be used for CPI
/// [`GatewayInstructon::ValidateMessage`] instruction.
///
/// This maps to [these lines in the Solidity Gateway](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/78fde453094074ca93ef7eea1e1395fba65ba4f6/contracts/AxelarGateway.sol#L636-L648)
#[repr(C)]
#[derive(Zeroable, Copy, Clone, PartialEq, Eq, Debug)]
pub enum MessageStatus {
    /// The state of the command after it has been approved
    Approved = 0,

    /// [`GatewayInstructon::ValidateMessage`] has been called and the command
    /// has been executed by the destination program.
    Executed = 1,
}

unsafe impl Pod for MessageStatus {}

/// Consruct a new Command ID.
/// The command id is used as a key for a message -- to prevent replay attacks.
/// It points to the storage slot that holds all metadata for a message.
///
/// For more info read [here](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/gateway/INTEGRATION.md#replay-prevention).
pub fn command_id(source_chain: &str, message_id: &str) -> [u8; 32] {
    solana_program::keccak::hashv(&[source_chain.as_bytes(), b"-", message_id.as_bytes()]).0
}
