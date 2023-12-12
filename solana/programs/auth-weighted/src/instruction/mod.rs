//! Instruction module; consist of fasade instructions, test ix constructors and
//! internal helpers.

pub mod validate;

use borsh::{to_vec, BorshDeserialize, BorshSerialize};

use crate::error::AuthWeightedError;

/// Instructions supported by the auth weighted program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum AuthWeightedInstruction {
    /// Instruction to Validate given message_hash and proof.
    ValidateProof,
}

impl<'a> AuthWeightedInstruction {
    /// [AuthWeightedInstruction] Deserialization.
    pub fn unpack(input: &'a [u8]) -> Result<Self, AuthWeightedError> {
        match Self::try_from_slice(input) {
            Ok(v) => Ok(v),
            Err(_) => Err(AuthWeightedError::InvalidInstruction),
        }
    }

    /// [AuthWeightedInstruction] Serialization.
    pub fn pack(&self) -> Vec<u8> {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        to_vec(&self).unwrap()
    }
}
