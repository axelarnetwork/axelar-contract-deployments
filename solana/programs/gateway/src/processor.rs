//! Program state processor

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use crate::events::emit_call_contract_event;
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
                msg!("Instruction: Queue");
            }
            GatewayInstruction::CallContract {
                sender,
                destination_chain,
                destination_contract_address,
                payload,
                payload_hash,
            } => {
                msg!("Instruction: CallContract");
                emit_call_contract_event(
                    &sender,
                    destination_chain,
                    destination_contract_address,
                    payload,
                    &payload_hash,
                )?
            }
        };
        Ok(())
    }
}
