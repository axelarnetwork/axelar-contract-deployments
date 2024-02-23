//! Program state processor

mod deploy_remote_interchain_token;
mod deploy_remote_token_manager;
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
            InterchainTokenServiceInstruction::DeployRemoteTokenManager {
                salt,
                destination_chain,
                token_manager_type,
                params,
                gas_value,
            } => Self::deploy_remote_token_manager(
                program_id,
                accounts,
                salt,
                destination_chain,
                token_manager_type,
                params,
                gas_value,
            ),
            InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
                salt,
                destination_chain,
                name,
                symbol,
                decimals,
                minter,
                gas_value,
            } => Self::deploy_remote_interchain_token(
                program_id,
                accounts,
                salt,
                destination_chain,
                name,
                symbol,
                decimals,
                minter,
                gas_value,
            ),
        }
    }
}
