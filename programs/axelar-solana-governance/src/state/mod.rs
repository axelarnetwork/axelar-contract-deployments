//! State related structs and operations for the governance contract.

use crate::seed_prefixes;
use borsh::{BorshDeserialize, BorshSerialize};
use core::any::type_name;
use core::mem::size_of;
use core::ops::RangeInclusive;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
};

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;

pub mod operator;
pub mod proposal;

type Hash = [u8; 32];
/// The [`solana_program::pubkey::Pubkey`] bytes.
type Address = [u8; 32];

/// The valid range for the proposal delay.
pub const VALID_PROPOSAL_DELAY_RANGE: RangeInclusive<u32> = 3600..=86400;

/// Governance configuration type.
#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
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
    /// Creates a new governance program config.
    #[must_use]
    pub const fn new(
        chain_hash: Hash,
        address_hash: Hash,
        minimum_proposal_eta_delay: u32,
        operator: Address,
    ) -> Self {
        Self {
            bump: 0, // This will be set by the program
            chain_hash,
            address_hash,
            minimum_proposal_eta_delay,
            operator,
        }
    }

    pub fn update(&mut self, mut update: GovernanceConfigUpdate) -> ProgramResult {
        if let Some(new_chain_hash) = update.chain_hash.take() {
            self.chain_hash = new_chain_hash;
        }

        if let Some(new_address_hash) = update.address_hash.take() {
            self.address_hash = new_address_hash;
        }

        if let Some(new_minimum_proposal_eta_delay) = update.minimum_proposal_eta_delay.take() {
            self.minimum_proposal_eta_delay = new_minimum_proposal_eta_delay;
        }
        validate_config(self)
    }

    /// Calculate governance config PDA
    #[must_use]
    pub fn pda() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seed_prefixes::GOVERNANCE_CONFIG], &crate::ID)
    }
}

/// Governance configuration update type.
#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct GovernanceConfigUpdate {
    pub chain_hash: Option<Hash>,
    pub address_hash: Option<Hash>,
    pub minimum_proposal_eta_delay: Option<u32>,
}

impl Sealed for GovernanceConfig {}

impl Pack for GovernanceConfig {
    const LEN: usize = size_of::<u8>()
        + size_of::<Hash>()
        + size_of::<Hash>()
        + size_of::<u32>()
        + size_of::<Address>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst)
            .expect("should pack GovernanceConfig into slice");
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!(
                "Error: failed to deserialize account as {}: {}",
                type_name::<Self>(),
                err
            );
            ProgramError::InvalidAccountData
        })
    }
}

pub(crate) fn validate_config(config: &GovernanceConfig) -> Result<(), ProgramError> {
    if !VALID_PROPOSAL_DELAY_RANGE.contains(&config.minimum_proposal_eta_delay) {
        msg!(
            "The minimum proposal ETA delay must be among {} and {} seconds",
            VALID_PROPOSAL_DELAY_RANGE.start(),
            VALID_PROPOSAL_DELAY_RANGE.end()
        );
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}
