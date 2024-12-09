//! State module contains data structures that keep state within the ITS
//! program.

use core::any::type_name;
use core::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::BorshPda;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

pub mod flow_limit;
pub mod token_manager;

/// Struct containing state of the ITS program.
#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub struct InterchainTokenService {
    /// Whether the ITS is paused.
    pub paused: bool,

    /// Bump used to derive the ITS PDA.
    pub bump: u8,
}

impl InterchainTokenService {
    /// The approximate length of the `InterchainTokenService` struct in bytes.
    /// Doesn't take padding into account.
    pub const LEN: usize = size_of::<bool>() + size_of::<u8>();

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

impl Pack for InterchainTokenService {
    const LEN: usize = size_of::<bool>() + size_of::<u8>();

    #[allow(clippy::unwrap_used)]
    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!(
                "Error: failed to deserialize account as {}: {}",
                type_name::<Self>(),
                err
            );
            ProgramError::InvalidAccountData
        })
    }
}

impl Sealed for InterchainTokenService {}
impl BorshPda for InterchainTokenService {}
