//! Program instructions processor.

use axelar_executable_old::{
    validate_message, AxelarCallableInstruction, PROGRAM_ACCOUNTS_START_INDEX,
};
use axelar_message_primitives::DataPayload;
use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::check_program_account;
use crate::instructions::encoding::MultiCallPayload;
use crate::instructions::MultiCallInstruction;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    ///
    /// # Errors
    ///
    /// A `ProgramError` containing the error that occurred is returned. Log
    /// messages are also generated with more detailed information.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        instruction_data: &[u8],
    ) -> ProgramResult {
        check_program_account(*program_id)?;

        let instruction = AxelarCallableInstruction::try_from_slice(instruction_data)?;

        match instruction {
            AxelarCallableInstruction::AxelarExecute(payload) => {
                validate_message(program_id, accounts, &payload)?;

                let (_multicall_program_accounts, target_programs_accounts) =
                    accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);
                let multicall_payload = MultiCallPayload::decode(
                    &payload.payload_without_accounts,
                    payload.encoding_scheme,
                )?;
                process_multicall(target_programs_accounts, multicall_payload)?;
            }
            AxelarCallableInstruction::Native(instruction) => {
                let instruction = MultiCallInstruction::try_from_slice(&instruction)?;
                let MultiCallInstruction::MultiCall { payload } = instruction;
                let decoded_payload = DataPayload::decode(&payload)?;
                let multicall_payload = MultiCallPayload::decode(
                    decoded_payload.payload_without_accounts(),
                    decoded_payload.encoding_scheme(),
                )?;

                process_multicall(accounts, multicall_payload)?;
            }
        }
        Ok(())
    }
}

fn process_multicall(
    accounts: &[AccountInfo<'_>],
    multicall_payload: MultiCallPayload,
) -> ProgramResult {
    for program_payload in multicall_payload.payloads {
        let program_account_index = program_payload.program_account_index;
        let Some(program_account) = accounts.get(program_account_index) else {
            msg!("Invalid program account index");
            return Err(ProgramError::InvalidArgument);
        };

        let start_index = program_payload.accounts_start_index;
        let end_index = program_payload.accounts_end_index;

        let Some(current_accounts) = accounts.get(start_index..end_index) else {
            msg!("Invalid account range");
            return Err(ProgramError::InvalidArgument);
        };

        let instruction = Instruction {
            program_id: *program_account.key,
            accounts: current_accounts
                .iter()
                .map(|account| AccountMeta {
                    pubkey: *account.key,
                    is_signer: account.is_signer,
                    is_writable: account.is_writable,
                })
                .collect(),
            data: borsh::to_vec(&AxelarCallableInstruction::Native(
                program_payload.instruction_data,
            ))?,
        };

        invoke(&instruction, current_accounts)?;
    }

    Ok(())
}
