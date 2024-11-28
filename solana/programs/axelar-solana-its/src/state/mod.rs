//! State module contains data structures that keep state within the ITS
//! program.

use core::mem::size_of;

use program_utils::StorableArchive;
use rkyv::{Archive, Deserialize, Serialize};

pub mod flow_limit;
pub mod token_manager;

/// Struct containing state of the ITS program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct InterchainTokenService {
    /// Whether the ITS is paused.
    pub paused: bool,

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
        Self {
            paused: false,
            bump,
        }
    }

    /// Pauses the Interchain Token Service.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpauses the Interchain Token Service.
    pub fn unpause(&mut self) {
        self.paused = false;
    }

    /// Returns the bump used to derive the ITS PDA.
    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }
}

impl StorableArchive<0> for InterchainTokenService {}
