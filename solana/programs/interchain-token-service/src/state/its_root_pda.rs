//! Solana state types for the Interchain Token Service

use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::{IsInitialized, Pack, Sealed};

/// Root PDA account for the Interchain Token Service
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct RootPDA {
    /// Discriminator for the PDA, used to ensure the account is initialized
    /// and we're not getting a bogus account
    discriminator: [u8; 8],
    /// Bump seed for the PDA
    pub bump_seed: u8,
}

impl RootPDA {
    const DISCRIMINATOR: &'static [u8; 8] = b"ITSRTPDA";

    /// Create a new `RootPDA` instance
    pub fn new(bump_seed: u8) -> Self {
        Self {
            discriminator: *Self::DISCRIMINATOR,
            bump_seed,
        }
    }
}

impl Sealed for RootPDA {}
impl Pack for RootPDA {
    const LEN: usize = size_of::<Self>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        borsh::to_writer(dst, self).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let root_pda = borsh::from_slice::<RootPDA>(src).unwrap();
        Ok(root_pda)
    }
}

impl IsInitialized for RootPDA {
    fn is_initialized(&self) -> bool {
        &self.discriminator == Self::DISCRIMINATOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_pda() {
        let root_pda = RootPDA::new(1);

        let mut dst = vec![0; RootPDA::LEN];
        RootPDA::pack(root_pda.clone(), &mut dst).unwrap();

        let unpacked = RootPDA::unpack(&dst).unwrap();
        assert_eq!(root_pda, unpacked);
    }
}
