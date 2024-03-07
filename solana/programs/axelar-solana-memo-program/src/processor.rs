//! Program state processor

use std::str::from_utf8;

use axelar_executable::{validate_contract_call, AxelarCallableInstruction};
use borsh::BorshDeserialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    // we allocate
    let payload = AxelarCallableInstruction::<()>::try_from_slice(input)?;
    let AxelarCallableInstruction::AxelarExecute(payload) = payload else {
        msg!("The memo program only accepts messages from Axelar");
        return Err(ProgramError::InvalidInstructionData);
    };
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
