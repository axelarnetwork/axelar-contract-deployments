//! State module for the Axelar Solana Gas Service

use anchor_discriminators_macros::account;
use bytemuck::{Pod, Zeroable};
use program_utils::pda::BytemuckedPda;
use solana_program::pubkey::Pubkey;

/// Keep track of the gas collector for aggregating gas payments
#[repr(C)]
#[account(zero_copy)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Config {
    /// Operator with permission to give refunds & withdraw funds
    pub operator: Pubkey,
    /// The bump seed used to derive the PDA, ensuring the address is valid.
    pub bump: u8,
}

impl BytemuckedPda for Config {}
