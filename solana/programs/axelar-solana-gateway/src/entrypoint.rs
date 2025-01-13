//! Program entrypoint

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use crate::processor::Processor;

#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
solana_program::entrypoint!(process_instruction);

/// Solana entrypoint
#[allow(clippy::missing_errors_doc)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    instruction_data: &[u8],
) -> ProgramResult {
    Processor::process_instruction(program_id, accounts, instruction_data)
}
