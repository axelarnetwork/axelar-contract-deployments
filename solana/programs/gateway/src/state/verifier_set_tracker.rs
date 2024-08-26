//! Module for the `VerifierSetTracker` account type.
use std::mem::size_of;

use axelar_message_primitives::command::U256;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

/// Ever-incrementing counter for keeping track of the sequence of signer sets
pub type Epoch = U256;
/// Verifier set hash
pub type VerifierSetHash = [u8; 32];

/// PDA that keeps track of core information about the verifier set.
/// We keep the track of the hash + epoch (sequential order of which verifier
/// set this is)
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct VerifierSetTracker {
    /// The canonical bump for this account.
    pub bump: u8,
    /// The epoch associated with this verifier set
    pub epoch: Epoch,
    /// The verifier set hash
    pub verifier_set_hash: VerifierSetHash,
}

impl std::fmt::Debug for VerifierSetTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifierSetTracker")
            .field("bump", &self.bump)
            .field("epoch", &self.epoch)
            .field("verifier_set_hash", &hex::encode(self.verifier_set_hash))
            .finish()
    }
}

impl Sealed for VerifierSetTracker {}

impl Pack for VerifierSetTracker {
    const LEN: usize = { size_of::<u8>() + size_of::<Epoch>() + size_of::<VerifierSetHash>() };

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
