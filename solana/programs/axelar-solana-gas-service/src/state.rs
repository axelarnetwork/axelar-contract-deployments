//! State module for the Axelar Solana Gas Service

use bytemuck::{Pod, Zeroable};
use program_utils::BytemuckedPda;
use solana_program::pubkey::Pubkey;

/// Keep track of the authority for aggregating gas payments
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Config {
    /// The authority with permission give refunds & withdraw funds
    pub authority: Pubkey,
    /// A 32-byte "salt" to ensure uniqueness in PDA derivation.
    pub salt: [u8; 32],
    /// The bump seed used to derive the PDA, ensuring the address is valid.
    pub bump: u8,
}

impl BytemuckedPda for Config {}
