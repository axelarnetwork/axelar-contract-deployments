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
#[repr(C)]
#[allow(clippy::partial_pub_fields)]
#[derive(Zeroable, Pod, Clone, Copy, PartialEq, Eq)]
pub struct VerifierSetTracker {
    /// The canonical bump for this account.
    pub bump: u8,
    /// Padding for the bump
    _padding: [u8; 7],
    /// The epoch associated with this verifier set
    pub epoch: Epoch,
    /// The verifier set hash
    pub verifier_set_hash: VerifierSetHash,
}

impl VerifierSetTracker {
    /// Create a new [`VerifierSetTracker`].
    #[must_use]
    pub const fn new(bump: u8, epoch: Epoch, verifier_set_hash: VerifierSetHash) -> Self {
        Self {
            bump,
            _padding: [0; 7],
            epoch,
            verifier_set_hash,
        }
    }
}

impl BytemuckedPda for VerifierSetTracker {}

impl core::fmt::Debug for VerifierSetTracker {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        fmt.debug_struct("VerifierSetTracker")
            .field("bump", &self.bump)
            .field("epoch", &self.epoch)
            .field("verifier_set_hash", &hex::encode(self.verifier_set_hash))
            .finish_non_exhaustive()
    }
}
