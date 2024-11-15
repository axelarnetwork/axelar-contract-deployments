#![deny(missing_docs)]

//! Program utility functions

use std::borrow::Borrow;
use std::io::Write;

use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::validation::validators::DefaultValidator;
use rkyv::{Archive, CheckBytes, Deserialize, Serialize};
use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
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

    /// Check if the account is an initialized PDA with a data check
    fn is_initialized_pda(&self, expected_owner_program_id: &Pubkey) -> bool;
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

/// Checks the rkyv encoded account program is initialised and
/// returns it's non-archived content.
pub fn check_rkyv_initialized_pda_non_archived<'a, T>(
    expected_owner_program_id: &Pubkey,
    acc_info: &'a AccountInfo,
    acc_data: &'a [u8],
) -> Result<T, ProgramError>
where
    T: Archive,
    T::Archived: 'a + CheckBytes<DefaultValidator<'a>> + Deserialize<T, SharedDeserializeMap>,
{
    acc_info.check_initialized_pda_without_deserialization(expected_owner_program_id)?;
    rkyv::from_bytes::<T>(acc_data).map_err(|err| {
        msg!("Cannot deserialize rkyv account data: {}", err);
        ProgramError::InvalidArgument
    })
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

#[cfg(test)]
#[allow(clippy::std_instead_of_core)]
#[allow(clippy::legacy_numeric_constants)]
mod tests {
    use std::u64;

    use super::*;

    #[test]
    fn u_256_le_conversion_to_i64() {
        let u256_t = from_u64_to_u256_le_bytes(u64::MAX);
        let conv = checked_from_u256_le_bytes_to_u64(&u256_t).unwrap();
        assert_eq!(u64::MAX, conv);
    }
}
