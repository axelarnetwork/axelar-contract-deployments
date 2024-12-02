//! Instruction types

use std::fmt::Debug;

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::instruction::Instruction;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum DummyGatewayInstruction {
    /// Prints the message back to the caller
    Echo {
        /// The message that's to be approved
        message: String,
    },
}

/// Creates a echo instruction.
pub fn echo(message: String) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![],
        data: to_vec(&DummyGatewayInstruction::Echo { message }).unwrap(),
    }
}
