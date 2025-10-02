//! Events emitted by the Axelar Solana Gas service

use anchor_discriminators::Discriminator;
use event_cpi_macros::event;
use solana_program::pubkey::Pubkey;

/// Represents the event emitted when native gas is paid for a contract call.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasPaidForContractCallEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// Destination chain on the Axelar network
    pub destination_chain: String,
    /// Destination address on the Axelar network
    pub destination_address: String,
    /// The payload hash for the event we're paying for
    pub payload_hash: [u8; 32],
    /// The refund address
    pub refund_address: Pubkey,
    /// The amount of SOL to send
    pub gas_fee_amount: u64,
}

/// Represents the event emitted when native gas is added.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasAddedEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// Index of the CallContract instruction
    pub ix_index: u8,
    /// Index of the CPI event inside inner instructions
    pub event_ix_index: u8,
    /// The refund address
    pub refund_address: Pubkey,
    /// amount of SOL
    pub gas_fee_amount: u64,
}

/// Represents the event emitted when native gas is refunded.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeGasRefundedEvent {
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// Index of the CallContract instruction
    pub ix_index: u8,
    /// Index of the CPI event inside inner instructions
    pub event_ix_index: u8,
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// amount of SOL
    pub fees: u64,
}

/// Represents the event emitted when an SPL token was used to pay for a contract call
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplGasPaidForContractCallEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// The Gas service config associated token account PDA
    pub config_pda_ata: Pubkey,
    /// Mint of the token
    pub mint: Pubkey,
    /// The token program id
    pub token_program_id: Pubkey,
    /// Destination chain on the Axelar network
    pub destination_chain: String,
    /// Destination address on the Axelar network
    pub destination_address: String,
    /// The payload hash for the event we're paying for
    pub payload_hash: [u8; 32],
    /// The refund address
    pub refund_address: Pubkey,
    /// The amount of tokens to send
    pub gas_fee_amount: u64,
}

/// Represents the event emitted when SPL token gas is added.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplGasAddedEvent {
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// The Gas service config associated token account PDA
    pub config_pda_ata: Pubkey,
    /// Mint of the token
    pub mint: Pubkey,
    /// The token program id
    pub token_program_id: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// Index of the CallContract instruction
    pub ix_index: u8,
    /// Index of the CPI event inside inner instructions
    pub event_ix_index: u8,
    /// The refund address
    pub refund_address: Pubkey,
    /// amount of tokens
    pub gas_fee_amount: u64,
}

/// Represents the event emitted when SPL token gas is refunded.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SplGasRefundedEvent {
    /// The Gas service config associated token account PDA
    pub config_pda_ata: Pubkey,
    /// Mint of the token
    pub mint: Pubkey,
    /// The token program id
    pub token_program_id: Pubkey,
    /// Solana transaction signature
    pub tx_hash: [u8; 64],
    /// The Gas service config PDA
    pub config_pda: Pubkey,
    /// Index of the CallContract instruction
    pub ix_index: u8,
    /// Index of the CPI event inside inner instructions
    pub event_ix_index: u8,
    /// The receiver of the refund
    pub receiver: Pubkey,
    /// amount of tokens
    pub fees: u64,
}
