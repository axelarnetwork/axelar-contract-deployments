//! Program state processor

mod execute;
mod give_token;
mod initialize;
mod take_token;

use borsh::BorshDeserialize;
use program_utils::check_program_account;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use crate::instruction::InterchainTokenServiceInstruction;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        check_program_account(program_id, crate::check_id)?;

        let instruction = InterchainTokenServiceInstruction::try_from_slice(input)?;

        match instruction {
            InterchainTokenServiceInstruction::Execute { payload } => {
                Self::execute(program_id, accounts, payload)
            }
            InterchainTokenServiceInstruction::Initialize {} => {
                Self::process_initialize(program_id, accounts)
            }
            InterchainTokenServiceInstruction::GiveToken {
                token_manager_type,
                amount,
            } => Self::give_token(program_id, accounts, token_manager_type, amount),
            InterchainTokenServiceInstruction::TakeToken {
                token_manager_type,
                amount,
            } => Self::take_token(program_id, accounts, token_manager_type, amount),
        }
    }
}
