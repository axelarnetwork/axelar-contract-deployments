//! Program state processor

use borsh::BorshDeserialize;
use program_utils::check_program_account;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;

use solana_program::account_info::next_account_info;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use event_cpi::Discriminator;
use event_cpi_macros::{emit_cpi, event, event_cpi_accounts, event_cpi_handler};

use crate::instruction::AxelarEventCpiInstruction;

#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
/// Event emitted when a memo is sent
pub struct MemoSentEvent {
    /// The sender of the memo
    pub sender: Pubkey,
    /// The memo content
    pub memo: String,
}

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id, crate::check_id)?;

    event_cpi_handler!(input);

    let instruction = AxelarEventCpiInstruction::try_from_slice(input)?;

    match instruction {
        AxelarEventCpiInstruction::EmitEvent { memo } => {
            process_memo(program_id, accounts, memo)?;
        }
    }

    Ok(())
}

fn process_memo(_program_id: &Pubkey, accounts: &[AccountInfo<'_>], memo: String) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let signer = next_account_info(account_info_iter)?;
    event_cpi_accounts!(account_info_iter);

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
    msg!("Memo: {}", memo);

    let event = MemoSentEvent {
        sender: *signer.key,
        memo: memo.clone(),
    };

    emit_cpi!(event);

    Ok(())
}
