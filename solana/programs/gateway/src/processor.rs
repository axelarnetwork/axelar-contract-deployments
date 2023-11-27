//! Program state processor

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;
use crate::instruction::GatewayInstruction;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let Ok(instruction) = GatewayInstruction::unpack(input) else {
            return Err(GatewayError::InvalidInstruction.into());
        };
        match instruction {
            GatewayInstruction::Queue { .. } => {
                msg!("Instruction: Queue")
            }
        };
        Ok(())
    }
}
