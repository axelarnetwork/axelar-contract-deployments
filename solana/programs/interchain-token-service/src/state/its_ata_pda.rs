use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::{IsInitialized, Pack, Sealed};

/// ITSATA PDA account for the Interchain Token Service
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ITSATAPDA {
    /// Discriminator for the PDA, used to ensure the account is initialized
    /// and we're not getting a bogus account
    discriminator: [u8; 8],
    /// Bump seed for the PDA
    pub bump_seed: u8,
}

impl Sealed for ITSATAPDA {}
impl Pack for ITSATAPDA {
    const LEN: usize = size_of::<Self>();

    fn pack_into_slice(&self, dst: &mut [u8]) {
        borsh::to_writer(dst, self).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let root_pda = borsh::from_slice::<ITSATAPDA>(src).unwrap();
        Ok(root_pda)
    }
}

impl ITSATAPDA {
    const DISCRIMINATOR: &'static [u8; 8] = b"ITSATAPD";

    /// Create a new `ITSATAPDA` instance
    pub fn new(bump_seed: u8) -> Self {
        Self {
            discriminator: *Self::DISCRIMINATOR,
            bump_seed,
        }
    }
}

impl IsInitialized for ITSATAPDA {
    fn is_initialized(&self) -> bool {
        &self.discriminator == Self::DISCRIMINATOR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_its_ata_pda() {
        let root_pda = ITSATAPDA::new(1);

        let mut dst = vec![0; ITSATAPDA::LEN];
        ITSATAPDA::pack(root_pda.clone(), &mut dst).unwrap();

        let unpacked = ITSATAPDA::unpack(&dst).unwrap();
        assert_eq!(root_pda, unpacked);
    }
}
