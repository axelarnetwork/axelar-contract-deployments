//! Program state processor

use std::str::from_utf8;

use axelar_executable::{
    validate_contract_call, AxelarCallableInstruction, AxelarExecutablePayload,
};
use borsh::BorshDeserialize;
use program_utils::check_program_account;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::AxelarMemoInstruction;

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id, crate::check_id)?;

    let payload = AxelarCallableInstruction::<AxelarMemoInstruction>::try_from_slice(input)?;
    match payload {
        AxelarCallableInstruction::AxelarExecute(payload) => {
            msg!("Instruction: AxelarExecute");
            process_message_from_axelar(program_id, accounts, payload)
        }
        AxelarCallableInstruction::Native(payload) => {
            msg!("Instruction: Native");
            process_native_ix(program_id, accounts, payload)
        }
    }
}

/// Process a message submitted by the relayer which originates from the Axelar
/// network
pub fn process_message_from_axelar(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: AxelarExecutablePayload,
) -> ProgramResult {
    validate_contract_call(program_id, accounts, &payload)?;

    let account_info_iter = &mut accounts.iter();
    let _gateway_approved_message_pda = next_account_info(account_info_iter)?;
    let _signing_pda = next_account_info(account_info_iter)?;
    let _gateway_root_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;

    // Iterate over the rest of the provided accounts
    for account_info in account_info_iter {
        // NOTE: The accounts WILL NEVER be signers, but they MAY be writable
        msg!(
            "Provided account {:?}-{}-{}",
            account_info.key,
            account_info.is_signer,
            account_info.is_writable
        );
    }

    let memo = from_utf8(&payload.payload_without_accounts).map_err(|err| {
        msg!("Invalid UTF-8, from byte {}", err.valid_up_to());
        ProgramError::InvalidInstructionData
    })?;
    msg!("Memo (len {}): {:?}", memo.len(), memo);

    Ok(())
}

/// Process a native instruction submitted by another program or user ON the
/// Solana network
pub fn process_native_ix(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    payload: AxelarMemoInstruction,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    match payload {
        AxelarMemoInstruction::SendToGateway {
            memo,
            destination_chain,
            destination_address,
        } => {
            msg!("Instruction: SendToGateway");
            let sender = next_account_info(account_info_iter)?;
            let gateway_root_pda = next_account_info(account_info_iter)?;
            invoke(
                &gateway::instructions::call_contract(
                    *gateway_root_pda.key,
                    *sender.key,
                    destination_chain,
                    destination_address,
                    memo.into_bytes(),
                )?,
                &[sender.clone(), gateway_root_pda.clone()],
            )?;
        }
    }

    Ok(())
}
