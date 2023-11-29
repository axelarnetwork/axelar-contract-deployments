//! Program state processor

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

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
        let instruction = GatewayInstruction::unpack(input)?;
        match instruction {
            GatewayInstruction::Queue { .. } => {
                msg!("Instruction: Queue")
            }
            GatewayInstruction::CallContract { .. } => {
                msg!("Instruction: CallContract")
            }
        };
        Ok(())
    }
}
