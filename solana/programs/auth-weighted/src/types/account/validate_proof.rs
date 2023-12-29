//! Validate Proof params account.

use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::AuthWeightedError;
use crate::types::proof::Proof;

/// [ValidateProofAccount]; Where instruction parameters are stored; message
/// hash and proof.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct ValidateProofAccount {
    /// Message hash / equivalent to EVM param.
    pub message_hash: [u8; 32],

    /// Proof / equivalent to EVM param.
    pub proof: Proof,
}

impl ValidateProofAccount {
    /// Checks if the proof is valid for the given message batch hash.
    pub fn validate(&self) -> Result<(), AuthWeightedError> {
        self.proof.validate(&self.message_hash)
    }
}
