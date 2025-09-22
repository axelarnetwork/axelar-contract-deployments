//! Module with data structure definition for approval of remote interchain token deployment.

use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::pda::BorshPda;

#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub(crate) struct DeployApproval {
    pub(crate) approved_destination_minter: [u8; 32],
    pub(crate) bump: u8,
}

impl BorshPda for DeployApproval {}
