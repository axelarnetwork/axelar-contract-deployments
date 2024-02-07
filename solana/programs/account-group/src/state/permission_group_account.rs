//! Permission group PDA account

use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

use crate::instruction::GroupId;

/// The operator group account is used as a top-level marker for
/// all the operator accounts
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct PermissionGroupAccount {
    /// Unique identifier for the permission group
    pub id: GroupId,
}

impl PermissionGroupAccount {
    /// Create a new operator group account
    pub fn new(id: GroupId) -> Self {
        Self { id }
    }
}

impl Sealed for PermissionGroupAccount {}
impl Pack for PermissionGroupAccount {
    const LEN: usize = size_of::<PermissionGroupAccount>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
