//! Transfer Operatorship params account.

use borsh::{to_vec, BorshDeserialize, BorshSerialize};

use crate::error::AuthWeightedError;
use crate::types::address::Address;
use crate::types::u256::U256;

/// [TransferOperatorshipAccount]; Where instruction parameters are stored.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct TransferOperatorshipAccount {
    /// List of operator addresses.
    pub operators: Vec<Address>,

    /// List of weights per operator.
    pub weights: Vec<U256>,

    /// Desired threshold.
    pub threshold: U256,
}

impl<'a> TransferOperatorshipAccount {
    /// Deserialize [TransferOperatorshipAccount].
    pub fn unpack(input: &'a [u8]) -> Result<Self, AuthWeightedError> {
        match Self::try_from_slice(input) {
            Ok(v) => Ok(v),
            Err(_) => Err(AuthWeightedError::MalformedTransferOperatorshipParams),
        }
    }

    /// Serialize [TransferOperatorshipAccount].
    pub fn pack(&self) -> Vec<u8> {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        to_vec(&self).unwrap()
    }
}

impl TransferOperatorshipAccount {
    /// Returns quantity of operators.
    pub fn operators_len(&self) -> usize {
        self.operators.len()
    }

    /// Returns quantity of weights.
    pub fn weights_len(&self) -> usize {
        self.weights.len()
    }
}

impl TransferOperatorshipAccount {
    /// Returns list of operators.
    pub fn operators(&self) -> &Vec<Address> {
        &self.operators
    }

    /// Returns list of weights.
    pub fn weights(&self) -> &Vec<U256> {
        &self.weights
    }

    /// Returns threshold.
    pub fn threshold(&self) -> &U256 {
        &self.threshold
    }
}
