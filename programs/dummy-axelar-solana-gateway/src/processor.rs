//! Program state processor.

use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use crate::instructions::DummyGatewayInstruction;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'_>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DummyGatewayInstruction::try_from_slice(input)?;
        match instruction {
            DummyGatewayInstruction::Echo { message } => {
                msg!("Echo: {}", message);
                Ok(())
            }
        }
    }
}
