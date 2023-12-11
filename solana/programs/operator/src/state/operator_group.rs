//! Operator group account

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

pub const OPERATOR_GROUP_TAG: &[u8; 16] = b"operator_group  ";

/// The operator group account is used as a top-level marker for
/// all the operator accounts
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct OperatorGroupAccount {
    tag: [u8; 16],
    id: [u8; 32],
}

impl OperatorGroupAccount {
    /// Create a new operator group account
    pub fn new(id: [u8; 32]) -> Self {
        Self {
            id,
            tag: *OPERATOR_GROUP_TAG,
        }
    }

    /// Make sure that the tag is present and correct
    pub fn is_initialized(&self) -> bool {
        self.tag == *OPERATOR_GROUP_TAG
    }
}

impl Sealed for OperatorGroupAccount {}
impl Pack for OperatorGroupAccount {
    const LEN: usize = 48;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let data = self.try_to_vec().unwrap();
        dst[..data.len()].copy_from_slice(&data);
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
