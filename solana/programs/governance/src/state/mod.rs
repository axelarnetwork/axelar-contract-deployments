//! State related structs and operations for the governance contract.

use core::mem::size_of;

use rkyv::{Archive, Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

use crate::seed_prefixes;

type Hash = [u8; 32];

/// Governance configuration type.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
#[repr(C)]
pub struct GovernanceConfig {
    /// The bump for this account.
    pub bump: u8,
    /// The name hash of the governance chain of the remote GMP contract. This
    /// param is used for validating the incoming GMP governance message.
    pub chain_hash: Hash,
    /// The address hash of the remote GMP governance contract. This param
    /// is used for validating the incoming GMP governance message.
    pub address_hash: Hash,
}

impl GovernanceConfig {
    /// Helps to pre-allocate the needed space when serializing.
    /// IMPORTANT: It must be kept updated with struct fields.
    pub const LEN: usize = size_of::<u8>() + size_of::<Hash>() + size_of::<Hash>();

    /// Creates a new governance program config.
    #[must_use]
    pub const fn new(bump: u8, chain_hash: Hash, address_hash: Hash) -> Self {
        Self {
            bump,
            chain_hash,
            address_hash,
        }
    }
    /// Calculate governance config PDA
    #[must_use]
    pub fn pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seed_prefixes::GOVERNANCE_CONFIG], &crate::ID)
    }
}
