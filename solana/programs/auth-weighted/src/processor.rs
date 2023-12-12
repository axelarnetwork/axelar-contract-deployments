//! Program state processor

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use crate::error::AuthWeightedError;
use crate::instruction::validate::validate_proof_ix;
use crate::instruction::AuthWeightedInstruction;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        _input: &[u8],
    ) -> ProgramResult {
        let Ok(instruction) = AuthWeightedInstruction::unpack(_input) else {
            return Err(AuthWeightedError::InvalidInstruction.into());
        };

        match instruction {
            AuthWeightedInstruction::ValidateProof => match validate_proof_ix(program_id, accounts)
            {
                Ok(_) => {}
                Err(e) => return Err(e),
            },
        };

        Ok(())
    }
}
