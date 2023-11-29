//! State structures for the interchain-address-tracker program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    msg,
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
    pubkey::Pubkey,
};

/// Account data.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Account {
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds.
    pub chain_name: String,
}

impl Sealed for Account {}
impl Pack for Account {
    const LEN: usize = 32 + 24;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        dst[..data.len()].copy_from_slice(&data);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
