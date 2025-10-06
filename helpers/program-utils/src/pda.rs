//! Contains

use borsh::to_vec;
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{AnyBitPattern, NoUninit};
use core::any::type_name;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program};
use std::borrow::Borrow;
use std::io::Write;

/// Initialize a PDA by writing borsh serialisable data to the buffer
pub fn init_pda<'a, 'b, T: solana_program::program_pack::Pack>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    data: T,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    prepare_account_structure(
        funder_info,
        to_create,
        program_id,
        system_program_info,
        T::get_packed_len() as u64,
        signer_seeds,
    )?;

    let mut account_data = to_create.try_borrow_mut_data()?;
    T::pack(data, &mut account_data)?;

    Ok(())
}

/// Prepare an account structure by transferring funds, allocating space, and assigning the program ID
fn prepare_account_structure<'a, 'b>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    space: u64,
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    // Calculate the minimum rent required for the account
    let rent = Rent::get()?.minimum_balance(space.try_into().expect("u64 fits into sbf word size"));

    // Check if the account already has enough lamports to cover rent, otherwise transfer
    if to_create.lamports() < rent {
        let required_funds_for_rent = rent
            .checked_sub(to_create.lamports())
            .expect("To not underflow when calculating needed rent");

        let transfer_ix =
            &system_instruction::transfer(funder_info.key, to_create.key, required_funds_for_rent);

        invoke_signed(
            transfer_ix,
            &[
                funder_info.clone(),
                to_create.clone(),
                system_program_info.clone(),
            ],
            &[signer_seeds],
        )?;
    };

    // Create the instructions to allocate space, and assign the program ID
    let alloc_ix = &system_instruction::allocate(to_create.key, space);
    let assign_ix = &system_instruction::assign(to_create.key, program_id);

    // Invoke the instructions to allocate space, and assign the program ID
    invoke_signed(
        alloc_ix,
        &[
            funder_info.clone(),
            to_create.clone(),
            system_program_info.clone(),
        ],
        &[signer_seeds],
    )?;
    invoke_signed(
        assign_ix,
        &[
            funder_info.clone(),
            to_create.clone(),
            system_program_info.clone(),
        ],
        &[signer_seeds],
    )
}

/// Initialize an associated account, with raw bytes.
pub fn init_pda_raw_bytes<'a, 'b>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    data: &[u8],
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
    init_pda_raw(
        funder_info,
        to_create,
        program_id,
        system_program_info,
        data.len() as u64,
        signer_seeds,
    )?;

    let mut account_data = to_create.try_borrow_mut_data()?;
    account_data.write_all(data).map_err(|err| {
        msg!("Cannot write data to account: {}", err);
        ProgramError::InvalidArgument
    })
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
    prepare_account_structure(
        funder_info,
        to_create,
        program_id,
        system_program_info,
        data_len,
        signer_seeds,
    )
}

/// Close an associated account
pub fn close_pda(
    lamport_destination: &AccountInfo<'_>,
    pda_to_close: &AccountInfo<'_>,
    expected_owner_program_id: &Pubkey,
) -> Result<(), solana_program::program_error::ProgramError> {
    // Ensure the PDA is initialized and owned by the expected program
    pda_to_close
        .check_initialized_pda_without_deserialization(expected_owner_program_id)
        .map_err(|err| {
            msg!("PDA is not initialized: {}", err);
            ProgramError::InvalidArgument
        })?;

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

/// Extension trait for AccountInfo to check if the account is an initialized
/// PDA
pub trait ValidPDA {
    /// Check if the account is an initialized PDA
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

    /// Check if the account is an initialized PDA with a data check
    fn is_initialized_pda(&self, expected_owner_program_id: &Pubkey) -> bool;

    /// Check if the account has meaningful data (i.e., not all zeros)
    fn has_meaningful_data(&self) -> bool;
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
            msg!("account does not have enough lamports");
            return Err(ProgramError::InsufficientFunds);
        }
        if !self.has_meaningful_data() {
            msg!("account does not have data or is all zeroed");
            return Err(ProgramError::InvalidAccountData);
        }
        let has_correct_owner = self.owner == expected_owner_program_id;
        if !has_correct_owner {
            msg!("account does not have the expected owner");
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

    fn is_initialized_pda(&self, expected_owner_program_id: &Pubkey) -> bool {
        let has_correct_owner = self.owner == expected_owner_program_id;
        self.has_meaningful_data() && has_correct_owner
    }

    fn has_meaningful_data(&self) -> bool {
        let data = self.try_borrow_data().expect("to borrow the data");
        if data.is_empty() {
            return false;
        }
        !is_zeroed(&data)
    }
}

/// Code borrowed from https://github.com/anza-xyz/agave/blob/master/transaction-context/src/lib.rs#L1068C1-L1078C2
fn is_zeroed(buf: &[u8]) -> bool {
    const ZEROS_LEN: usize = 1024;
    const ZEROS: [u8; ZEROS_LEN] = [0; ZEROS_LEN];
    let mut chunks = buf.chunks_exact(ZEROS_LEN);

    #[allow(clippy::indexing_slicing)]
    {
        chunks.all(|chunk| chunk == &ZEROS[..])
            && chunks.remainder() == &ZEROS[..chunks.remainder().len()]
    }
}
/// Convenience trait to store and load borsh serialized data to/from an account.
pub trait BorshPda
where
    Self: Sized + Clone + BorshSerialize + BorshDeserialize,
{
    /// Initializes an account with the current object serialized data.
    fn init<'a>(
        &self,
        program_id: &Pubkey,
        system_account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        into: &AccountInfo<'a>,
        signer_seeds: &[&[u8]],
    ) -> ProgramResult {
        let serialized_data = to_vec(self)?;

        init_pda_raw_bytes(
            payer,
            into,
            program_id,
            system_account,
            &serialized_data,
            signer_seeds,
        )?;

        Ok(())
    }

    /// Stores the current object serialized data into the destination account.
    /// The account must have been initialized beforehand.
    fn store<'a>(
        &self,
        payer: &AccountInfo<'a>,
        destination: &AccountInfo<'a>,
        system_program: &AccountInfo<'a>,
    ) -> ProgramResult {
        let serialized_data = to_vec(self)?;

        if serialized_data.len() > destination.data_len() {
            let lamports_needed = Rent::get()?.minimum_balance(serialized_data.len());
            let lamports_diff = lamports_needed.saturating_sub(destination.lamports());

            if lamports_diff > 0 {
                invoke(
                    &system_instruction::transfer(payer.key, destination.key, lamports_diff),
                    &[payer.clone(), destination.clone(), system_program.clone()],
                )?;
            }
        }

        destination.realloc(serialized_data.len(), false)?;
        let mut account_data = destination.try_borrow_mut_data()?;
        account_data.copy_from_slice(serialized_data.as_slice());

        Ok(())
    }

    /// Loads the account data and deserializes it.
    fn load(source_account: &AccountInfo<'_>) -> Result<Self, ProgramError> {
        let account_data = source_account.try_borrow_data()?;
        let deserialized = match Self::try_from_slice(&account_data[..]) {
            Ok(value) => value,
            Err(err) => {
                msg!(
                    "Warning: failed to deserialize account as {}: {}. The account might not have been initialized.",
                    type_name::<Self>(),
                    err,
                );

                return Err(ProgramError::from(err));
            }
        };

        Ok(deserialized)
    }
}

/// A trait for types that can be safely converted to and from byte slices using `bytemuck`.
pub trait BytemuckedPda: Sized + NoUninit + AnyBitPattern {
    /// Reads an immutable reference to `Self` from a byte slice.
    ///
    /// This method attempts to interpret the provided byte slice as an instance of `Self`.
    /// It checks that the length of the slice matches the size of `Self` to ensure safety.
    fn read(data: &[u8]) -> Option<&Self> {
        let result: &Self = bytemuck::try_from_bytes(data)
            .map_err(|err| {
                msg!("bytemuck error {:?}", err);
                err
            })
            .ok()?;
        Some(result)
    }

    /// Reads a mutable reference to `Self` from a mutable byte slice.
    ///
    /// Similar to [`read`], but allows for mutation of the underlying data.
    /// This is useful when you need to modify the data in place.
    fn read_mut(data: &mut [u8]) -> Option<&mut Self> {
        let result: &mut Self = bytemuck::try_from_bytes_mut(data)
            .map_err(|err| {
                msg!("bytemuck error {:?}", err);
                err
            })
            .ok()?;
        Some(result)
    }

    /// Writes the instance of `Self` into a mutable byte slice.
    ///
    /// This method serializes `self` into its byte representation and copies it into the
    /// provided mutable byte slice. It ensures that the destination slice is of the correct
    /// length to hold the data.
    fn write(&self, data: &mut [u8]) -> Option<()> {
        let self_bytes = bytemuck::bytes_of(self);
        if data.len() != self_bytes.len() {
            return None;
        }
        data.copy_from_slice(self_bytes);
        Some(())
    }
}

/// Defines "Info" and "Meta" structs for easier account array handling.
/// These help ensuring consistency between the client and the program.
#[macro_export]
macro_rules! account_array_structs {
    (
        $info_struct_name:ident,
        $meta_struct_name:ident,
        $(
            $(#[$attr:meta])*
            $field_name:ident
        ),*
    ) => {
        struct $info_struct_name <'a, 'b> {
            $(
                $(#[$attr])*
                pub $field_name : &'b solana_program::account_info::AccountInfo<'a>,
            )*
        }

        impl<'a, 'b> $info_struct_name <'a, 'b> {
            fn from_account_iter<I>(iter: &mut I) -> Result<Self, solana_program::program_error::ProgramError>
            where
                I: Iterator<Item = &'b solana_program::account_info::AccountInfo<'a>>,
            {
                let result = Self {
                    $(
                        $field_name: solana_program::account_info::next_account_info(iter)?,
                    )*
                };

                Ok(result)
            }
        }

        pub struct $meta_struct_name {
            $(
                $(#[$attr])*
                pub $field_name : solana_program::instruction::AccountMeta,
            )*
        }

        impl $meta_struct_name {
            pub fn to_account_vec(self) -> Vec<solana_program::instruction::AccountMeta> {
                vec![
                    $(
                        self.$field_name,
                    )*
                ]
            }
        }
    };
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn is_zeroed_empty() {
        assert!(is_zeroed(&[]));
    }

    #[test]
    fn is_zeroed_small_all_zero() {
        assert!(is_zeroed(&[0u8; 17]));
    }

    #[test]
    fn is_zeroed_small_non_zero() {
        let mut v = vec![0u8; 17];
        v[5] = 1;
        assert!(!is_zeroed(&v));
    }

    #[test]
    fn is_zeroed_exact_chunk() {
        assert!(is_zeroed(&vec![0u8; 1024]));
    }

    #[test]
    fn is_zeroed_exact_chunk_non_zero() {
        let mut v = vec![0u8; 1024];
        v[1023] = 9;
        assert!(!is_zeroed(&v));
    }

    #[test]
    fn is_zeroed_multi_chunk_all_zero() {
        assert!(is_zeroed(&vec![0u8; 1024 * 3 + 11]));
    }

    #[test]
    fn is_zeroed_multi_chunk_non_zero_middle_chunk() {
        let mut v = vec![0u8; 1024 * 2 + 11];
        v[1024 + 123] = 2;
        assert!(!is_zeroed(&v));
    }

    #[test]
    fn is_zeroed_just_below_chunk_all_zero() {
        assert!(is_zeroed(&vec![0u8; 1023]));
    }

    #[test]
    fn is_zeroed_just_below_chunk_non_zero() {
        let mut v = vec![0u8; 1023];
        v[1022] = 1;
        assert!(!is_zeroed(&v));
    }

    #[test]
    fn is_zeroed_just_above_chunk_all_zero() {
        assert!(is_zeroed(&vec![0u8; 1025]));
    }

    #[test]
    fn is_zeroed_just_above_chunk_non_zero_last() {
        let mut v = vec![0u8; 1025];
        v[1024] = 3;
        assert!(!is_zeroed(&v));
    }

    #[test]
    fn is_zeroed_large_all_zero() {
        assert!(is_zeroed(&vec![0u8; 1024 * 5]));
    }

    #[test]
    fn is_zeroed_large_last_byte_non_zero() {
        let mut v = vec![0u8; 1024 * 5];
        let last = v.len() - 1;
        v[last] = 0xFF;
        assert!(!is_zeroed(&v));
    }

    #[test]
    fn is_zeroed_large_only_first_byte_non_zero() {
        let mut v = vec![0u8; 1024 * 4 + 17];
        v[0] = 0xAB;
        assert!(!is_zeroed(&v));
    }
}
