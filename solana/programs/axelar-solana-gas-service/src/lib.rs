//! Axelar Gas Service program for the Solana blockchain
#![allow(clippy::little_endian_bytes)]
pub mod entrypoint;
pub mod instructions;
pub mod processor;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("gasHQkvaC4jTD2MQpAuEN3RdNwde2Ym5E5QNDoh6m6G");

/// Seed prefixes for PDAs created by this program
pub mod seed_prefixes {
    /// The seed used when deriving the configuration PDA.
    pub const CONFIG_SEED: &[u8] = b"gas-service";
}

/// Event discriminators (prefixes) used to identify logged events related to native gas operations.
pub mod event_prefixes {
    /// Prefix emitted when native gas is paid for a contract call.
    pub const NATIVE_GAS_PAID_FOR_CONTRACT_CALL: &[u8] = b"native gas paid for contract call";
    /// Prefix emitted when native gas is added to an already emtted contract call.
    pub const NATIVE_GAS_ADDED: &[u8] = b"native gas added";
    /// Prefix emitted when native gas is refunded.
    pub const NATIVE_GAS_REFUNDED: &[u8] = b"native gas refunded";
}

/// Checks that the provided `program_id` matches the current program’s ID.
///
/// # Errors
///
/// - if the provided `program_id` does not match.
#[inline]
pub fn check_program_account(program_id: Pubkey) -> Result<(), ProgramError> {
    if program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Derives the configuration PDA for this program.
///
/// Given a `program_id`, a `salt` (32-byte array), and an `authority` (`Pubkey`), this function
/// uses [`Pubkey::find_program_address`] to return the derived PDA and its associated bump seed.
#[inline]
#[must_use]
pub fn get_config_pda(program_id: &Pubkey, salt: &[u8; 32], authority: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::CONFIG_SEED, salt, authority.as_ref()],
        program_id,
    )
}

/// Checks that the given `expected_pubkey` matches the derived PDA for the provided parameters.
///
/// # Panics
/// - if the seeds + bump don't result in a valid PDA
///
/// # Errors
///
/// - if the derived PDA does not match the `expected_pubkey`.
#[inline]
#[track_caller]
pub fn assert_valid_config_pda(
    bump: u8,
    salt: &[u8; 32],
    authority: &Pubkey,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = Pubkey::create_program_address(
        &[
            seed_prefixes::CONFIG_SEED,
            salt,
            authority.as_ref(),
            &[bump],
        ],
        &crate::ID,
    )
    .expect("invalid bump for the config pda");

    if &derived_pubkey == expected_pubkey {
        Ok(())
    } else {
        msg!("Error: Invalid Config PDA ");
        Err(ProgramError::IncorrectProgramId)
    }
}

/// Utilities for working with gas service events
pub mod event_utils {

    /// Errors that may occur while parsing a `MessageEvent`.
    #[derive(Debug, thiserror::Error)]
    pub enum EventParseError {
        /// Occurs when a required field is missing in the event data.
        #[error("Missing data: {0}")]
        MissingData(&'static str),

        /// The data is there but it's not of valid format
        #[error("Invalid data: {0}")]
        InvalidData(&'static str),

        /// Occurs when the length of a field does not match the expected length.
        #[error("Invalid length for {field}: expected {expected}, got {actual}")]
        InvalidLength {
            /// the field that we're trying to parse
            field: &'static str,
            /// the desired length
            expected: usize,
            /// the actual length
            actual: usize,
        },

        /// Occurs when a field contains invalid UTF-8 data.
        #[error("Invalid UTF-8 in {field}: {source}")]
        InvalidUtf8 {
            /// the field we're trying to parse
            field: &'static str,
            /// underlying error
            #[source]
            source: std::string::FromUtf8Error,
        },

        /// Generic error for any other parsing issues.
        #[error("Other error: {0}")]
        Other(&'static str),
    }

    pub(crate) fn read_array<const N: usize>(
        field: &'static str,
        data: &[u8],
    ) -> Result<[u8; N], EventParseError> {
        if data.len() != N {
            return Err(EventParseError::InvalidLength {
                field,
                expected: N,
                actual: data.len(),
            });
        }
        let array = data
            .try_into()
            .map_err(|_err| EventParseError::InvalidLength {
                field,
                expected: N,
                actual: data.len(),
            })?;
        Ok(array)
    }

    pub(crate) fn read_string(
        field: &'static str,
        data: Vec<u8>,
    ) -> Result<String, EventParseError> {
        String::from_utf8(data).map_err(|err| EventParseError::InvalidUtf8 { field, source: err })
    }

    #[allow(clippy::little_endian_bytes)]
    pub(crate) fn parse_u64_le(field: &'static str, data: &[u8]) -> Result<u64, EventParseError> {
        if data.len() != 8 {
            return Err(EventParseError::InvalidData(field));
        }
        Ok(u64::from_le_bytes(data.try_into().expect("length checked")))
    }
}
