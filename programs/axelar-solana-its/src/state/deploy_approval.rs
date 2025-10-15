//! Module with data structure definition for approval of remote interchain token deployment.

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;
use program_utils::pda::BorshPda;

#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
pub(crate) struct DeployApproval {
    pub(crate) approved_destination_minter: [u8; 32],
    pub(crate) bump: u8,
}

impl BorshPda for DeployApproval {}
