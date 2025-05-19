//! All PDAs owned by the memo program
use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::{Pack, Sealed};

/// A counter PDA that keeps track of how many memos have been received from the
/// gateway
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Counter {
    /// the counter of how many memos have been received from the gateway
    pub counter: u64,
    /// Bump for the counter PDA
    pub bump: u8,
}

impl Pack for Counter {
    const LEN: usize = size_of::<u64>() + size_of::<u8>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        borsh::to_writer(dst, self).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let counter_pda = borsh::from_slice::<Counter>(src).unwrap();
        Ok(counter_pda)
    }
}

impl Sealed for Counter {}
