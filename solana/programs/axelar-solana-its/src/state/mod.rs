//! State module contains data structures that keep state within the ITS
//! program.

use core::mem::size_of;

use rkyv::{Archive, Deserialize, Serialize};

pub mod flow_limit;
pub mod token_manager;

/// Struct containing state of the ITS program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct InterchainTokenService {
    /// Bump used to derive the ITS PDA.
    pub bump: u8,
}

impl InterchainTokenService {
    /// The approximate length of the `InterchainTokenService` struct in bytes.
    /// Doesn't take padding into account.
    pub const LEN: usize = size_of::<u8>();

    /// Create a new `InterchainTokenService` instance.
    #[must_use]
    pub const fn new(bump: u8) -> Self {
        Self { bump }
    }
}

impl ArchivedInterchainTokenService {
    /// Deserializes the `InterchainTokenService` from the given bytes using
    /// `rkyv` for zero-copy deserialization.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> &Self {
        // SAFETY: The data is assumed to be a valid archived `InterchainTokenService`.
        // The `TokenManager` is always serialized as an archived struct.
        unsafe { rkyv::archived_root::<InterchainTokenService>(data) }
    }
}
