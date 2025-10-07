//! Events emitted by the Axelar Solana Gas service

use anchor_discriminators::Discriminator;
use event_cpi_macros::event;
use solana_program::pubkey::Pubkey;

/// Represents the event emitted when gas is paid for a contract call.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasPaidEvent {
    /// The sender/payer of gas
    pub sender: Pubkey,
    /// Destination chain on the Axelar network
    pub destination_chain: String,
    /// Destination address on the Axelar network
    pub destination_address: String,
    /// The payload hash for the event we're paying for
    pub payload_hash: [u8; 32],
    /// The amount paid
    pub amount: u64,
    /// The refund address
    pub refund_address: Pubkey,
    //
    // SPL token fields
    //
    /// Mint of the token
    pub mint: Option<Pubkey>,
    /// Token program id
    pub token_program_id: Option<Pubkey>,
    /// Sender token account
    pub sender_token_account: Option<Pubkey>,
}

/// Represents the event emitted when gas is added.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasAddedEvent {
    /// The sender/payer of gas
    pub sender: Pubkey,
    /// Message Id
    pub message_id: String,
    /// The amount added
    pub amount: u64,
    /// The refund address
    pub refund_address: Pubkey,
    //
    // SPL token fields
    //
    /// Mint of the token
    pub mint: Option<Pubkey>,
    /// Token program id
    pub token_program_id: Option<Pubkey>,
    /// Sender token account
    pub sender_token_account: Option<Pubkey>,
}

/// Represents the event emitted when gas is refunded.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasRefundedEvent {
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// Message Id
    pub message_id: String,
    /// The amount refunded
    pub amount: u64,
    //
    // SPL token fields
    //
    /// Mint of the token
    pub mint: Option<Pubkey>,
    /// Token program id
    pub token_program_id: Option<Pubkey>,
    /// Receiver token account
    pub receiver_token_account: Option<Pubkey>,
}

/// Represents the event emitted when accumulated gas is collected.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GasCollectedEvent {
    /// The receiver of the gas
    pub receiver: Pubkey,
    /// The amount collected
    pub amount: u64,
    //
    // SPL token fields
    //
    /// Mint of the token
    pub mint: Option<Pubkey>,
    /// Token program id
    pub token_program_id: Option<Pubkey>,
    /// Receiver token account
    pub receiver_token_account: Option<Pubkey>,
}
