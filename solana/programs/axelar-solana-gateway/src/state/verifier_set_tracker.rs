//! Module for the `VerifierSetTracker` account type.

use axelar_message_primitives::U256;
use bytemuck::{Pod, Zeroable};
use program_utils::BytemuckedPda;

/// Ever-incrementing counter for keeping track of the sequence of signer sets
pub type Epoch = U256;
/// Verifier set hash
pub type VerifierSetHash = [u8; 32];

/// PDA that keeps track of core information about the verifier set.
/// We keep the track of the hash + epoch (sequential order of which verifier
/// set this is)
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct VerifierSetTracker {
    /// The canonical bump for this account.
    pub bump: u8,
    /// Padding for the bump
    pub _padding: [u8; 7],
    /// The epoch associated with this verifier set
    pub epoch: Epoch,
    /// The verifier set hash
    pub verifier_set_hash: VerifierSetHash,
}

impl BytemuckedPda for VerifierSetTracker {}

impl std::fmt::Debug for VerifierSetTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VerifierSetTracker")
            .field("bump", &self.bump)
            .field("epoch", &self.epoch)
            .field("verifier_set_hash", &hex::encode(self.verifier_set_hash))
            .finish()
    }
}
