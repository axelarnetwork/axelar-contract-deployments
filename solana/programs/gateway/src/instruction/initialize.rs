//! Initialize Gateway root PDA.

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;

use crate::{check_initialized, check_program_account, cmp_addr, find_root_pda};
pub(crate) fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> Result<(), ProgramError> {
    check_program_account(*program_id)?;

    let accounts_iter = &mut accounts.iter();

    let payer_info = next_account_info(accounts_iter)?;
    let pda_info = next_account_info(accounts_iter)?;
    let system_program_info = next_account_info(accounts_iter)?;

    let (expected_pda_info, bump) = find_root_pda();

    cmp_addr(pda_info, expected_pda_info)?;
    check_initialized(pda_info.lamports())?;

    let rent = Rent::get()?;
    let ix = &system_instruction::create_account(
        payer_info.key,
        pda_info.key,
        rent.minimum_balance(data.len().max(1)),
        data.len() as u64,
        &crate::id(),
    );
    invoke_signed(
        ix,
        &[
            payer_info.clone(),
            pda_info.clone(),
            system_program_info.clone(),
        ],
        &[&[&[bump]]],
    )?;

    let mut account_data = pda_info.try_borrow_mut_data()?;
    account_data[..data.len()].copy_from_slice(data);

    Ok(())
}
