//! State module contains data structures that keep state within the ITS
//! program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

/// Struct containing state of the ITS program.
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct InterchainTokenService {
    /// Bump used to derive the ITS PDA.
    pub bump: u8,
}

impl Sealed for InterchainTokenService {}

#[allow(clippy::expect_used)]
impl Pack for InterchainTokenService {
    const LEN: usize = core::mem::size_of::<Self>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst)
            .expect("InterchainTokenService state serialization failed");
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
