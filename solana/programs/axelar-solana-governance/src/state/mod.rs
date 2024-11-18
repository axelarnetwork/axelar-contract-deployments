//! State related structs and operations for the governance contract.

use core::mem::size_of;

use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

use crate::seed_prefixes;

pub mod operator;
pub mod proposal;

type Hash = [u8; 32];
/// The [`solana_program::pubkey::Pubkey`] bytes.
type Address = [u8; 32];

/// Governance configuration type.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
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
    /// This is the minimum time in seconds from `now()` a proposal can
    /// be executed. If the incoming GMP proposal does not have an ETA
    /// superior to `unix_timestamp` + `this field`, such ETA will be
    /// will be set as new ETA.
    pub minimum_proposal_eta_delay: u32,
    /// The pub key of the operator. This address is able to execute proposals
    /// that were previously scheduled by the Axelar governance infrastructure
    /// via GMP flow regardless of the proposal ETA.
    pub operator: Address,
}

impl GovernanceConfig {
    /// Helps to pre-allocate the needed space when serializing.
    /// IMPORTANT: It must be kept updated with struct fields.
    pub const LEN: usize = size_of::<u8>()
        + size_of::<Hash>()
        + size_of::<Hash>()
        + size_of::<u32>()
        + size_of::<Address>();

    /// Creates a new governance program config.
    #[must_use]
    pub const fn new(
        bump: u8,
        chain_hash: Hash,
        address_hash: Hash,
        minimum_proposal_eta_delay: u32,
        operator: Address,
    ) -> Self {
        Self {
            bump,
            chain_hash,
            address_hash,
            minimum_proposal_eta_delay,
            operator,
        }
    }
    /// Calculate governance config PDA
    #[must_use]
    pub fn pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seed_prefixes::GOVERNANCE_CONFIG], &crate::ID)
    }
}
