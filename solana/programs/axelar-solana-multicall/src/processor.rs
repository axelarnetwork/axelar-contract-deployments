//! Program instructions processor.

use axelar_executable::{validate_message, AxelarMessagePayload, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
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

        if let Some(message) =
            axelar_executable::parse_axelar_message(instruction_data).transpose()?
        {
            msg!("Instruction: AxelarExecute");
            validate_message(accounts, &message)?;

            let (protocol_accounts, target_programs_accounts) =
                accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);
            let message_payload_account = protocol_accounts
                .get(1)
                .ok_or(ProgramError::NotEnoughAccountKeys)?;
            let account_data = message_payload_account.try_borrow_data()?;
            let message_payload: ImmutMessagePayload<'_> = (**account_data).try_into()?;
            let axelar_payload = AxelarMessagePayload::decode(message_payload.raw_payload)?;
            let payload = axelar_payload.payload_without_accounts();
            let multicall_payload =
                MultiCallPayload::decode(payload, axelar_payload.encoding_scheme())?;

            return process_multicall(target_programs_accounts, multicall_payload);
        }

        msg!("Instruction: Native");
        let instruction = MultiCallInstruction::try_from_slice(instruction_data)?;
        let MultiCallInstruction::MultiCall { payload } = instruction;
        let decoded_payload = AxelarMessagePayload::decode(&payload)?;
        let multicall_payload = MultiCallPayload::decode(
            decoded_payload.payload_without_accounts(),
            decoded_payload.encoding_scheme(),
        )?;

        process_multicall(accounts, multicall_payload)?;

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
            data: program_payload.instruction_data,
        };

        invoke(&instruction, current_accounts)?;
    }

    Ok(())
}
