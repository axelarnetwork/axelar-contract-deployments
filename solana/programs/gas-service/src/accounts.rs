//! Structs for Gas Service program accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use gateway::types::PubkeyWrapper;

/// Root PDA type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GasServiceRootPDA {
    authority: PubkeyWrapper,
}

impl GasServiceRootPDA {
    /// Creates a new
    pub fn new(authority: PubkeyWrapper) -> Self {
        Self { authority }
    }

    /// Returns true, in case of authority key stored in account match the one
    /// from argument.
    pub fn check_authority(self, authority: PubkeyWrapper) -> bool {
        authority == self.authority
    }
}
