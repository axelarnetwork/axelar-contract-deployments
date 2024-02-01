//! Solana state types for the Interchain Token Service

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::{Pack, Sealed};

/// Root PDA account for the Interchain Token Service
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct RootPDA {}

impl Sealed for RootPDA {}
impl Pack for RootPDA {
    const LEN: usize = 0;

    fn pack_into_slice(&self, _dst: &mut [u8]) {
        // No data to pack
    }

    fn unpack_from_slice(_src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        // No data to unpack
        Ok(RootPDA {})
    }
}
