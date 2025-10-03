#![cfg(test)]

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;
use borsh::{from_slice, to_vec};
use bytemuck::{Pod, Zeroable};
use program_utils::pda::BytemuckedPda;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

/// Keep track of the gas collector for aggregating gas payments
#[repr(C)]
#[account]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Config {
    /// Operator with permission to give refunds & withdraw funds
    pub operator: Pubkey,
    /// The bump seed used to derive the PDA, ensuring the address is valid.
    pub bump: u8,
}

impl BytemuckedPda for Config {}

#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
/// Struct containing flow information for a specific epoch.
pub struct FlowState {
    pub flow_limit: Option<u64>,
    pub flow_in: u64,
    pub flow_out: u64,
    pub epoch: u64,
}

#[test]
#[allow(clippy::indexing_slicing)]
fn test_account_serde_bytemuck() {
    let config = Config {
        operator: Pubkey::new_unique(),
        bump: 1,
    };
    let bytes = to_vec(&config).unwrap();
    assert_eq!(&bytes[..8], Config::DISCRIMINATOR);
    let deserialized: Config = from_slice(&bytes).unwrap();
    assert_eq!(config, deserialized);
}

#[test]
#[allow(clippy::indexing_slicing)]
fn test_account_serde() {
    let flow = FlowState {
        flow_limit: Some(100),
        flow_in: 50,
        flow_out: 30,
        epoch: 1,
    };
    let bytes = to_vec(&flow).unwrap();
    assert_eq!(&bytes[..8], FlowState::DISCRIMINATOR);
    let deserialized: FlowState = from_slice(&bytes).unwrap();
    assert_eq!(flow, deserialized);
}
