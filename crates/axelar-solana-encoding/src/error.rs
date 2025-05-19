//! Common Error type used within the encoding crate

/// Common Error type used within the encoding crate.
#[derive(Debug, thiserror::Error)]
pub enum EncodingError {
    /// Indicates that an attempt was made to merkelise an empty verifier set.
    #[error("Empty verifier set")]
    CannotMerkeliseEmptyVerifierSet,

    /// Indicates that an attempt was made to merkelise an empty message batch.
    #[error("Empty message batch")]
    CannotMerkeliseEmptyMessageSet,

    /// Indicates that the set to be merkelised exceeds the allowable size.
    #[error("The set that needs to be merkelised is too large")]
    SetSizeTooLarge,

    /// Represents I/O related errors (usually encoding related)
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
}
