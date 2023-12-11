//! Operator account

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum OperatorAccountType {
    #[default]
    Active = 1,
    Inactive = 2,
}

/// The operator account is used to make sure that the account is
/// indeed an operator
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct OperatorAccount {
    operator_type: OperatorAccountType,
}

impl OperatorAccount {
    /// Create a new operator account
    pub fn new_active() -> Self {
        Self {
            operator_type: OperatorAccountType::Active,
        }
    }

    /// Make the operator account inactive
    pub fn make_inactive(&mut self) {
        self.operator_type = OperatorAccountType::Inactive;
    }

    /// Make the operator account active
    pub fn make_active(&mut self) {
        self.operator_type = OperatorAccountType::Active;
    }

    /// Make sure that the account is active
    pub fn is_active(&self) -> bool {
        self.operator_type == OperatorAccountType::Active
    }
}

impl Sealed for OperatorAccount {}
impl Pack for OperatorAccount {
    const LEN: usize = 1;

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
