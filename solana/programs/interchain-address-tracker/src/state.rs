//! State structures for the interchain-address-tracker program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

/// Registered Chain Account data.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct RegisteredChainAccount {
    /// The owner of this account.
    pub owner: Pubkey,
    /// The amount of tokens this account holds. Must be limited to 64 bytes.
    pub chain_name: String,
}

impl RegisteredChainAccount {
    /// Creates a new RegisteredChainAccount.
    pub fn try_to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut buffer: Vec<u8> = Vec::new();
        self.serialize(&mut buffer)?;
        Ok(buffer)
    }
}

impl Sealed for RegisteredChainAccount {}
impl Pack for RegisteredChainAccount {
    const LEN: usize = 32 + 64; // 64 bytes for the chain name

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

/// Registered Trusted Address Account.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct RegisteredTrustedAddressAccount {
    /// The trusted address of the remote chain. Limited to 64 bytes.
    pub address: String,
}

impl RegisteredTrustedAddressAccount {
    /// Creates a new RegisteredTrustedAddressAccount.
    pub fn try_to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut buffer: Vec<u8> = Vec::new();
        self.serialize(&mut buffer).unwrap();
        Ok(buffer)
    }
}

impl Sealed for RegisteredTrustedAddressAccount {}
impl Pack for RegisteredTrustedAddressAccount {
    const LEN: usize = 64;

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
