//! Program state accounts.

use std::collections::BTreeMap;

use borsh::{to_vec, BorshDeserialize, BorshSerialize};

use crate::error::AuthWeightedError;
use crate::types::u256::U256;

/// [AuthWeightedStateAccount]; where program keeps operators and epoch.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct AuthWeightedStateAccount {
    /// solidity: uint256 public currentEpoch;
    pub current_epoch: U256,

    /// solidity: mapping(bytes32 => uint256) public epochForHash;
    pub epoch_for_hash: BTreeMap<[u8; 32], U256>,

    /// solidity: mapping(uint256 => bytes32) public hashForEpoch;
    pub hash_for_epoch: BTreeMap<U256, [u8; 32]>,
}

impl<'a> AuthWeightedStateAccount {
    /// Deserialize [AuthWeightedStateAccount].
    pub fn unpack(input: &'a [u8]) -> Result<Self, AuthWeightedError> {
        match Self::try_from_slice(input) {
            Ok(v) => Ok(v),
            Err(_) => Err(AuthWeightedError::MalformedState),
        }
    }

    /// Serialize [AuthWeightedStateAccount].
    pub fn pack(&self) -> Vec<u8> {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        to_vec(&self).unwrap()
    }
}
