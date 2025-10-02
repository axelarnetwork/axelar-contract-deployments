#![cfg(test)]

use anchor_discriminators_macros::AccountDiscriminator;
use borsh::BorshDeserialize;
use bytemuck::{Pod, Zeroable};
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

/// Keep track of the gas collector for aggregating gas payments
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug, AccountDiscriminator)]
pub struct Config {
    /// Operator with permission to give refunds & withdraw funds
    pub operator: Pubkey,
    /// The bump seed used to derive the PDA, ensuring the address is valid.
    pub bump: u8,
}

impl BytemuckedPda for Config {}

#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize, AccountDiscriminator)]
/// Struct containing flow information for a specific epoch.
pub struct FlowState {
    pub flow_limit: Option<u64>,
    pub flow_in: u64,
    pub flow_out: u64,
    pub epoch: u64,
}
