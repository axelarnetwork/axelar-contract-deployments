//! Validate Proof params account.

use borsh::{to_vec, BorshDeserialize, BorshSerialize};

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

impl<'a> ValidateProofAccount {
    /// Deserialize [ValidateProofAccount].
    pub fn unpack(input: &'a [u8]) -> Result<Self, AuthWeightedError> {
        match Self::try_from_slice(input) {
            Ok(v) => Ok(v),
            Err(_) => Err(AuthWeightedError::MalformedProof),
        }
    }

    /// Serialize [ValidateProofAccount].
    pub fn pack(&self) -> Vec<u8> {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        to_vec(&self).unwrap()
    }
}
