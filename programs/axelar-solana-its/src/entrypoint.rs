//! Program entrypoint

#![cfg(not(feature = "no-entrypoint"))]

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use crate::processor;

solana_program::entrypoint!(process_instruction);

fn process_instruction<'a: 'b, 'b>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'b>],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}
