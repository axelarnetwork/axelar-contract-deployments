#![deny(missing_docs)]

//! Program utility functions

use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;

/// Initialize an associated account
pub fn init_pda<'a, 'b, T: solana_program::program_pack::Pack + borsh::BorshSerialize>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    data: T,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let rent = Rent::get()?;
    let ix = &system_instruction::create_account(
        funder_info.key,
        to_create.key,
        rent.minimum_balance(T::LEN).max(1),
        T::get_packed_len() as u64,
        program_id,
    );
    invoke_signed(
        ix,
        &[
            funder_info.clone(),
            to_create.clone(),
            system_program_info.clone(),
        ],
        &[signer_seeds],
    )?;
    let mut account_data = to_create.try_borrow_mut_data()?;
    let serialized_data = data.try_to_vec().unwrap();
    account_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    Ok(())
}

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey, check_f: fn(&Pubkey) -> bool) -> ProgramResult {
    if !&check_f(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}
