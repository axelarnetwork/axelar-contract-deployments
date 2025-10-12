//! State module contains data structures that keep state within the ITS
//! program.

use std::collections::HashSet;

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;
use program_utils::pda::BorshPda;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;

pub mod deploy_approval;
pub mod flow_limit;
pub mod interchain_transfer_execute;
pub mod token_manager;

/// Struct containing state of the ITS program.
#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct InterchainTokenService {
    /// The address of the Axelar ITS Hub contract.
    pub its_hub_address: String,
    /// Name of the chain ITS is running on.
    pub chain_name: String,

    /// Whether the ITS is paused.
    pub paused: bool,

    /// Trusted chains
    pub trusted_chains: HashSet<String>,

    /// Bump used to derive the ITS PDA.
    pub bump: u8,
}

impl InterchainTokenService {
    /// Create a new `InterchainTokenService` instance.
    #[must_use]
    pub fn new(bump: u8, chain_name: String, its_hub_address: String) -> Self {
        Self {
            its_hub_address,
            chain_name,
            paused: false,
            trusted_chains: HashSet::new(),
            bump,
        }
    }

    /// Pauses the Interchain Token Service.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpauses the Interchain Token Service.
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Returns the bump used to derive the ITS PDA.
    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }

    /// Add a chain as trusted
    pub fn add_trusted_chain(&mut self, chain_id: String) {
        self.trusted_chains.insert(chain_id);
    }

    /// Remove a chain from trusted
    pub fn remove_trusted_chain(&mut self, chain_id: &str) -> ProgramResult {
        if !self.trusted_chains.remove(chain_id) {
            msg!("Chain '{}' is not in the trusted chains list", chain_id);
            return Err(ProgramError::InvalidArgument);
        }

        Ok(())
    }

    /// Checks whether or not a given chain is trusted
    #[must_use]
    pub fn is_trusted_chain(&self, chain_id: &str) -> bool {
        self.trusted_chains.contains(chain_id)
    }
}

impl BorshPda for InterchainTokenService {}
