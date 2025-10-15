#![deny(missing_docs)]

//! Program utility functions

use solana_program::account_info::AccountInfo;
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_program, sysvar};

pub mod pda;
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

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey, check_f: fn(&Pubkey) -> bool) -> ProgramResult {
    if !&check_f(program_id) {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Tries to fetch the next account as an optional account from the account iterator
///
/// By convention, when an account is set to be the `program_id`, it's interpreted as
/// [`Option::None`]
pub fn next_optional_account_info<'a, 'b, I: Iterator<Item = &'a AccountInfo<'b>>>(
    iter: &mut I,
    program_id: &Pubkey,
) -> Result<Option<I::Item>, ProgramError> {
    iter.next()
        .ok_or(ProgramError::NotEnoughAccountKeys)
        .map(|account| {
            if account.key == program_id {
                None
            } else {
                Some(account)
            }
        })
}

/// Macro to ensure exactly one feature from a list is enabled
#[macro_export]
macro_rules! ensure_single_feature {
    ($($feature:literal),+) => {
        // Check that at least one feature is enabled
        #[cfg(not(any($(feature = $feature),+)))]
        compile_error!(concat!("Exactly one of these features must be enabled: ", $(stringify!($feature), ", "),+));

        // Generate all pair combinations to check mutual exclusivity
        ensure_single_feature!(@pairs [] $($feature),+);
    };

    // Helper to generate all pairs
    (@pairs [$($processed:literal),*] $first:literal $(,$rest:literal)*) => {
        // Check current element against all processed elements
        $(
            #[cfg(all(feature = $first, feature = $processed))]
            compile_error!(concat!("Features '", $first, "' and '", $processed, "' are mutually exclusive"));
        )*

        // Continue with the rest
        ensure_single_feature!(@pairs [$($processed,)* $first] $($rest),*);
    };

    // Base case: no more elements to process
    (@pairs [$($processed:literal),*]) => {};
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
