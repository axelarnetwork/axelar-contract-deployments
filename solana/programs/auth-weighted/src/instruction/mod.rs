//! Instruction module; consist of fasade instructions, test ix constructors and
//! internal helpers.

pub mod validate;

use borsh::{BorshDeserialize, BorshSerialize};

/// Instructions supported by the auth weighted program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum AuthWeightedInstruction {
    /// Instruction to Validate given message_hash and proof.
    ValidateProof,
}
