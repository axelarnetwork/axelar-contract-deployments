//! Structs for Gas Service program accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Root PDA type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GasServiceRootPDA {
    authority: Pubkey,
}

impl GasServiceRootPDA {
    /// Creates a new
    pub fn new(authority: Pubkey) -> Self {
        Self { authority }
    }

    /// Returns true, in case of authority key stored in account match the one
    /// from argument.
    pub fn check_authority(self, authority: Pubkey) -> bool {
        authority == self.authority
    }
}
