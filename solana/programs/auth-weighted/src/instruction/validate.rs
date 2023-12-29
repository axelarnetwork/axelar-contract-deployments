//! Proof validation logic

use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::AuthWeightedInstruction;

/// Creates a validate proof instruction.
/// Thats purely for testing purposes.
pub fn validate_proof(
    payer: &Pubkey,
    params: &Pubkey,
    state: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*params, false),
        AccountMeta::new_readonly(*state, false),
    ];

    let data = borsh::to_vec(&AuthWeightedInstruction::ValidateProof)?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
