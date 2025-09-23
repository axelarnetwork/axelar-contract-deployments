//! Instruction module for the Axelar Memo program.

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
pub use solana_program;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Instructions supported by the Axelar Event CPI program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum AxelarEventCpiInstruction {
    /// Emit an event with a memo
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [s] payer
    EmitEvent {
        /// The memo string to be emitted with the event
        memo: String,
    },
}

/// Creates a [`AxelarEventCpiInstruction::EmitEvent`] instruction.
pub fn emit_event(payer: &Pubkey, memo: String) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarEventCpiInstruction::EmitEvent { memo })?;

    let accounts = vec![AccountMeta::new(*payer, true)];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}
