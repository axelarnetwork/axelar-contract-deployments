//! Utilities for parsing events emitted by the `GasService` program.

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

pub(crate) fn read_string(field: &'static str, data: Vec<u8>) -> Result<String, EventParseError> {
    String::from_utf8(data).map_err(|err| EventParseError::InvalidUtf8 { field, source: err })
}

#[allow(clippy::little_endian_bytes)]
pub(crate) fn parse_u64_le(field: &'static str, data: &[u8]) -> Result<u64, EventParseError> {
    if data.len() != 8 {
        return Err(EventParseError::InvalidData(field));
    }
    Ok(u64::from_le_bytes(data.try_into().expect("length checked")))
}
