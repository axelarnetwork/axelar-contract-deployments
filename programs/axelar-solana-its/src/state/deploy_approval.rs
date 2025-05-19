//! Module with data structure definition for approval of remote interchain token deployment.
use core::any::type_name;
use core::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::BorshPda;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub(crate) struct DeployApproval {
    pub(crate) approved_destination_minter: [u8; 32],
    pub(crate) bump: u8,
}

impl Pack for DeployApproval {
    const LEN: usize = size_of::<u8>() + size_of::<[u8; 32]>();

    #[allow(clippy::unwrap_used)]
    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!(
                "Error: failed to deserialize account as {}: {}",
                type_name::<Self>(),
                err
            );
            ProgramError::InvalidAccountData
        })
    }
}
impl Sealed for DeployApproval {}
impl BorshPda for DeployApproval {}
