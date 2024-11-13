#![deny(missing_docs)]

//! Program utility functions

use std::borrow::Borrow;
use std::io::Write;

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Serialize};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program};

/// mini helper to log from native Rust or to the program log
/// Very useful for debugging when you have to run some code on Solana and via
/// native Rust
#[macro_export]
macro_rules! log_everywhere {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);

        #[cfg(not(target_os = "solana"))]
        {
            dbg!(message);
        }

        #[cfg(target_os = "solana")]
        {
            solana_program::msg!("SOL: {}", message);
        }
    }}
}

/// Initialize a PDA by writing borsh serialisable data to the buffer
// TODO add constraint that the T: IsInitialized + Pack + BorshSerialize
pub fn init_pda<'a, 'b, T: solana_program::program_pack::Pack>(
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
    T::pack(data, &mut account_data)?;
    Ok(())
}

/// Initializes a PDA without writing anything to the data storage
pub fn init_pda_raw<'a, 'b>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    data_len: u64,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    let rent = Rent::get()?;
    let ix = &system_instruction::create_account(
        funder_info.key,
        to_create.key,
        rent.minimum_balance(data_len.try_into().expect("u64 fits into sbf word size"))
            .max(1),
        data_len,
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
    Ok(())
}

/// Initialize an associated account
pub fn init_rkyv_pda<'a, 'b, const N: usize, T>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    rkyv_type: T,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError>
where
    T: Serialize<AllocSerializer<N>>,
{
    let data = rkyv::to_bytes::<_, N>(&rkyv_type).map_err(|err| {
        msg!("Cannot serialize rkyv account data: {}", err);
        ProgramError::InvalidArgument
    })?;

    let rent = Rent::get()?;
    let ix = &system_instruction::create_account(
        funder_info.key,
        to_create.key,
        rent.minimum_balance(data.len()).max(1),
        data.len() as u64,
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
    account_data.write_all(&data)?;
    Ok(())
}

/// Close an associated account
pub fn close_pda(
    lamport_destination: &AccountInfo<'_>,
    pda_to_close: &AccountInfo<'_>,
) -> Result<(), solana_program::program_error::ProgramError> {
    // Transfer the lamports to the destination account
    let dest_starting_lamports = lamport_destination.lamports();
    **lamport_destination.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(pda_to_close.lamports())
        .unwrap();
    **pda_to_close.lamports.borrow_mut() = 0;

    // Downgrade the PDA's account to the system program
    pda_to_close.assign(&system_program::ID);

    // Downsize the PDA's account to 0
    pda_to_close.realloc(0, false)?;

    Ok(())
}

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey, check_f: fn(&Pubkey) -> bool) -> ProgramResult {
    if !&check_f(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Extension trait for AccountInfo to check if the account is an initialized
/// PDA
pub trait ValidPDA {
    /// Check if the account is an initialized PDA
    // TODO add constraint that the T: IsInitialized + Pack + BorshSerialize
    fn check_initialized_pda<T: solana_program::program_pack::Pack>(
        &self,
        expected_owner_program_id: &Pubkey,
    ) -> Result<T, ProgramError>;

    /// Check if the account is an initialized PDA without deserializing the
    /// data
    fn check_initialized_pda_without_deserialization(
        &self,
        expected_owner_program_id: &Pubkey,
    ) -> Result<(), ProgramError>;

    /// Check if the account is an initialized PDA
    fn check_uninitialized_pda(&self) -> Result<(), ProgramError>;
}

/// Checks the rkyv encoded account program is initialised and
/// returns it's content.
pub fn check_rkyv_initialized_pda<'a, T: Archive>(
    expected_owner_program_id: &Pubkey,
    acc_info: &'a AccountInfo,
    acc_data: &'a [u8],
) -> Result<&'a T::Archived, ProgramError> {
    acc_info.check_initialized_pda_without_deserialization(expected_owner_program_id)?;
    Ok(unsafe { rkyv::archived_root::<T>(acc_data) })
}

impl<'a> ValidPDA for &AccountInfo<'a> {
    fn check_initialized_pda<T: solana_program::program_pack::Pack>(
        &self,
        expected_owner_program_id: &Pubkey,
    ) -> Result<T, ProgramError> {
        self.check_initialized_pda_without_deserialization(expected_owner_program_id)?;

        let data = self.try_borrow_data()?;
        T::unpack_from_slice(data.borrow()).map_err(|_| ProgramError::InvalidAccountData)
    }

    fn check_initialized_pda_without_deserialization(
        &self,
        expected_owner_program_id: &Pubkey,
    ) -> Result<(), ProgramError> {
        let has_lamports = **self.try_borrow_lamports()? > 0;
        if !has_lamports {
            return Err(ProgramError::InsufficientFunds);
        }
        let has_correct_owner = self.owner == expected_owner_program_id;
        if !has_correct_owner {
            return Err(ProgramError::IllegalOwner);
        }

        Ok(())
    }

    fn check_uninitialized_pda(&self) -> Result<(), ProgramError> {
        let data_is_empty = self.try_borrow_data()?.is_empty();
        if !data_is_empty {
            return Err(ProgramError::InvalidAccountData);
        }
        let owner_is_system = self.owner == &solana_program::system_program::id();
        if !owner_is_system {
            return Err(ProgramError::IllegalOwner);
        }
        Ok(())
    }
}
