#![deny(missing_docs)]

//! Program utility functions

use core::any::type_name;
use std::borrow::Borrow;
use std::io::Write;

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use bytemuck::{AnyBitPattern, NoUninit};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program, sysvar};

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

/// Initialize an associated account, with raw bytes.
pub fn init_pda_raw_bytes<'a, 'b>(
    funder_info: &'a AccountInfo<'b>,
    to_create: &'a AccountInfo<'b>,
    program_id: &Pubkey,
    system_program_info: &'a AccountInfo<'b>,
    data: &[u8],
    signer_seeds: &[&[u8]],
) -> Result<(), ProgramError> {
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

    /// Check if the account is an initialized PDA with a data check
    fn is_initialized_pda(&self, expected_owner_program_id: &Pubkey) -> bool;
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
        let data_is_empty = self
            .try_borrow_data()
            .expect("to borrow the data")
            .is_empty();
        let has_correct_owner = self.owner == expected_owner_program_id;
        !data_is_empty && has_correct_owner
    }
}

/// Converts a little-endian 256-bit unsigned integer to a 64-bit unsigned
/// integer.
///
/// This function takes a 32-byte array representing a little-endian 256-bit
/// unsigned integer and converts it to a 64-bit unsigned integer, ensuring that
/// the upper bytes are all zero.
///
/// # Arguments
///
/// * `le_u256` - A reference to a 32-byte array representing a little-endian
///   256-bit unsigned integer.
///
/// # Errors
///
/// Returns [`ProgramError::InvalidArgument`] if the upper bytes are not all
/// zero and the caller should take care of adding more contextual information
/// into the program.
///
/// # Examples
///
/// ```
/// use program_utils::checked_from_u256_le_bytes_to_u64;
///
/// let le_u256: [u8; 32] = [0; 32];
/// let result = checked_from_u256_le_bytes_to_u64(&le_u256).unwrap();
/// assert_eq!(result, 0);
/// ```
#[allow(clippy::little_endian_bytes)]
pub fn checked_from_u256_le_bytes_to_u64(le_u256: &[u8; 32]) -> Result<u64, ProgramError> {
    // Check that the upper bytes are all zero.
    if le_u256[8..32].iter().any(|&byte| byte != 0) {
        return Err(ProgramError::InvalidArgument);
    }

    // Copy the first 8 bytes into a u64 array.
    let mut u64data: [u8; 8] = [0_u8; 8];
    u64data.copy_from_slice(&le_u256[0..8]);

    // Convert the array to a u64.
    Ok(u64::from_le_bytes(u64data))
}

/// Converts from little endian u64 type to [u8; 32] type
#[must_use]
#[allow(clippy::little_endian_bytes)]
pub fn from_u64_to_u256_le_bytes(u64: u64) -> [u8; 32] {
    let mut u256 = [0; 32];
    u256[0..8].copy_from_slice(&u64.to_le_bytes());
    u256
}

/// Returns the current Unix timestamp as a `u64`.
///
/// This function normalizes the timestamp to `u64` because we do not care about
/// time before the epoch, and it helps to standardize our data types.
///
/// # Errors
///
/// Returns `ProgramError::InvalidArgument` if the conversion from `i64` to
/// `u64` fails.
pub fn current_time() -> Result<u64, ProgramError> {
    let solana_now = Clock::get()?.unix_timestamp;
    let now = u64::try_from(solana_now).map_err(|err| {
        msg!("Cannot convert solana type into u64: {}", err);
        ProgramError::InvalidArgument
    })?;
    Ok(now)
}

/// Transfers lamports from one account to another.
///
/// # Arguments
///
/// * `from_account` - The account to transfer lamports from.
/// * `to_account` - The account to transfer lamports to.
/// * `amount_of_lamports` - The amount of lamports to transfer.
///
/// # Errors
///
/// Returns `ProgramError::InsufficientFunds` if the `from_account` does not
/// have enough lamports.
pub fn transfer_lamports(
    from_account: &AccountInfo<'_>,
    to_account: &AccountInfo<'_>,
    amount_of_lamports: u64,
) -> ProgramResult {
    // Does the from account have enough lamports to transfer?
    if **from_account.try_borrow_lamports()? < amount_of_lamports {
        return Err(ProgramError::InsufficientFunds);
    }
    // Debit from_account and credit to_account
    let mut from_account = from_account.try_borrow_mut_lamports()?;
    let mut to_account = to_account.try_borrow_mut_lamports()?;
    **from_account = from_account
        .checked_sub(amount_of_lamports)
        .ok_or(ProgramError::InsufficientFunds)?;
    **to_account = to_account
        .checked_add(amount_of_lamports)
        .ok_or(ProgramError::InsufficientFunds)?;
    Ok(())
}

/// Convenience trait to store and load rkyv serialized data to/from an account.
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

            invoke(
                &system_instruction::transfer(payer.key, destination.key, lamports_diff),
                &[payer.clone(), destination.clone(), system_program.clone()],
            )?;
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

/// Checks if the key is from system account
pub fn validate_system_account_key(key: &Pubkey) -> Result<(), ProgramError> {
    if !system_program::check_id(key) {
        msg!("Wrong system account key");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the key is from spl associated token account
pub fn validate_spl_associated_token_account_key(key: &Pubkey) -> Result<(), ProgramError> {
    if !spl_associated_token_account::check_id(key) {
        msg!("Wrong spl associated token account key");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the key is from rent
pub fn validate_rent_key(key: &Pubkey) -> Result<(), ProgramError> {
    if !sysvar::rent::check_id(key) {
        msg!("Wrong rent key");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the key is from sysvar instructions
pub fn validate_sysvar_instructions_key(key: &Pubkey) -> Result<(), ProgramError> {
    if !sysvar::instructions::check_id(key) {
        msg!("Wrong sysvar instructions key");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the key is from sysvar instructions
pub fn validate_mpl_token_metadata_key(key: &Pubkey) -> Result<(), ProgramError> {
    if *key != mpl_token_metadata::ID {
        msg!("Wrong mpl token metadata key");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use solana_program::{system_program, sysvar};

    use crate::{
        validate_mpl_token_metadata_key, validate_rent_key,
        validate_spl_associated_token_account_key, validate_system_account_key,
        validate_sysvar_instructions_key,
    };

    #[test]
    fn test_validate_system_account_key() {
        assert!(validate_system_account_key(&system_program::ID).is_ok());
        assert!(validate_system_account_key(&sysvar::instructions::ID).is_err());
        assert!(validate_system_account_key(&sysvar::rent::ID).is_err());
    }

    #[test]
    fn test_validate_spl_associated_token_account_key() {
        assert!(
            validate_spl_associated_token_account_key(&spl_associated_token_account::ID).is_ok()
        );
        assert!(validate_spl_associated_token_account_key(&sysvar::instructions::ID).is_err());
        assert!(validate_spl_associated_token_account_key(&sysvar::rent::ID).is_err());
    }

    #[test]
    fn test_validate_rent_key() {
        assert!(validate_rent_key(&sysvar::rent::ID).is_ok());
        assert!(validate_rent_key(&system_program::ID).is_err());
        assert!(validate_rent_key(&spl_associated_token_account::ID).is_err());
    }

    #[test]
    fn test_sysvar_instructions_key() {
        assert!(validate_sysvar_instructions_key(&sysvar::instructions::ID).is_ok());
        assert!(validate_sysvar_instructions_key(&system_program::ID).is_err());
        assert!(validate_sysvar_instructions_key(&spl_associated_token_account::ID).is_err());
    }

    #[test]
    fn test_mpl_token_metadata_key() {
        assert!(validate_mpl_token_metadata_key(&mpl_token_metadata::ID).is_ok());
        assert!(validate_mpl_token_metadata_key(&system_program::ID).is_err());
        assert!(validate_mpl_token_metadata_key(&spl_associated_token_account::ID).is_err());
    }
}
