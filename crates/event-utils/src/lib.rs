//! This crate provides the `Event` trait to be used by Solana programs.
//! It also provides utility functions to read data from the decoded logs emitted by the program.
//!
//! This crate should be used together with the `event-macros` crate, which provides a derive macro
//! to implement the [`Event`] trait for compoatible structs.
//!
//! The `Event` is emitted and parsed differently compared to how Anchor does it, where the
//! structures are serialized and deserialized with `Borsh`. Here we simply use `sol_log_data` to
//! log each field as bytes, `sol_log_data` then logs these bytes as base64 encoded strings.
//!
//! All programs within the `solana-axelar` integration are encourage to use this crate to emit
//! events as JavaScript utilities are also provided to parse them from logs.
use axelar_message_primitives::U256;
use solana_program::pubkey::Pubkey;

pub use base64;
pub use event_macros::*;

/// Trait for structs that represent events which can be emitted and deserialized from Solana logs.
pub trait Event {
    /// Discriminator associated with this event type.
    const DISC: &'static [u8; 16];

    /// Emits the event data using `sol_log_data`
    fn emit(&self);

    /// Tries to parse an event of this type from a log message string.
    ///
    /// # Errors
    ///
    /// In case the event could not be parsed from the log message.
    fn try_from_log(log: &str) -> Result<Self, EventParseError>
    where
        Self: Sized;

    /// Parses an event of this type from combined, decoded log data bytes.
    /// Assumes the discriminant has *already been checked* by the caller.
    ///
    ///
    /// # Errors
    ///
    /// In case the iterator doesn't yield the expected fields or the data present cannot be
    /// deserialized into the expected fields.
    fn deserialize<I: Iterator<Item = Vec<u8>>>(data: I) -> Result<Self, EventParseError>
    where
        Self: Sized;
}

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

/// Tries to read a fixed-size array from the provided data slice.
///
/// # Errors
///
/// In case the size of `data` doesn't match the expected array size.
pub fn read_array<const N: usize>(
    field: &'static str,
    data: &[u8],
) -> Result<[u8; N], EventParseError> {
    let array = data
        .try_into()
        .map_err(|_err| EventParseError::InvalidLength {
            field,
            expected: N,
            actual: data.len(),
        })?;

    Ok(array)
}

/// Tries to read a [`String`] from the provided data vector.
///
/// # Errors
///
/// In case the data cannot be converted into a valid UTF-8 string.
pub fn read_string(field: &'static str, data: Vec<u8>) -> Result<String, EventParseError> {
    String::from_utf8(data).map_err(|err| EventParseError::InvalidUtf8 { field, source: err })
}

/// Tries to read a [`Pubkey`] from the provided data slice.
///
/// # Errors
///
/// In case the size of `data` doesn't match the expected length of a [`Pubkey`].
pub fn read_pubkey(field: &'static str, data: &[u8]) -> Result<Pubkey, EventParseError> {
    let bytes = data
        .try_into()
        .map_err(|_err| EventParseError::InvalidLength {
            field,
            expected: 32,
            actual: data.len(),
        })?;

    Ok(Pubkey::new_from_array(bytes))
}

/// Tries to read a [`Vec<u8>`] from the provided data slice.
///
/// # Errors
///
/// In case the size of `data` doesn't match the expected length of a [`bool`].
pub fn read_bool(field: &'static str, data: &[u8]) -> Result<bool, EventParseError> {
    if data.len() != 1 {
        return Err(EventParseError::InvalidData(field));
    }

    let byte = *data.first().ok_or(EventParseError::InvalidLength {
        field,
        expected: 1,
        actual: data.len(),
    })?;

    Ok(byte != 0)
}

macro_rules! make_read_functions {
    ( $( ($fn_name:ident, $ty:ty) ),* $(,)? ) => {
        $(
            /// Tries to read a fixed-size value from the provided data slice.
            ///
            /// # Errors
            ///
            /// In case the data cannot be converted into the specified type.
            #[allow(clippy::little_endian_bytes)]
            pub fn $fn_name(field: &'static str, data: &[u8]) -> Result<$ty, EventParseError> {
                const SIZE: usize = core::mem::size_of::<$ty>();

                let bytes: [u8; SIZE] = data.try_into().map_err(|_err| {
                    EventParseError::InvalidLength {
                        field,
                        expected: SIZE,
                        actual: data.len(),
                    }
                })?;

                Ok(<$ty>::from_le_bytes(bytes))
            }
        )*
    }
}

make_read_functions! {
    (read_u8, u8),
    (read_u16, u16),
    (read_u32, u32),
    (read_u64, u64),
    (read_u128, u128),
    (read_i8, i8),
    (read_i16, i16),
    (read_i32, i32),
    (read_i64, i64),
    (read_i128, i128),
    (read_f32, f32),
    (read_f64, f64),
    (read_u256, U256),
}
