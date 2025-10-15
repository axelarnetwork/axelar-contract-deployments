//! State module contains data structures that keep state within the ITS
//! program.

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;
use program_utils::pda::BorshPda;

/// Signed PDA to prove that ITS called an executable indeed. Only stores it's bump.
#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct InterchainTransferExecute {
    /// The interchain transfer execute PDA bump seed.
    pub bump: u8,
}

impl InterchainTransferExecute {
    /// Creates a new `TokenManager` struct.
    #[must_use]
    pub const fn new(bump: u8) -> Self {
        Self { bump }
    }
}

impl BorshPda for InterchainTransferExecute {}
