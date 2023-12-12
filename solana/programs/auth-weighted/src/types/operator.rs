//! Operator types.

use borsh::{to_vec, BorshDeserialize, BorshSerialize};

use super::address::Address;
use super::u256::U256;
use crate::error::AuthWeightedError;

/// [Operators] consist of public keys of signers, weights (bond) and desired
/// threshold.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct Operators {
    /// List of addresses; look [Address].
    addresses: Vec<Address>,

    /// List of weights / bond.
    weights: Vec<U256>,

    /// Desired treshold.
    threshold: U256,
}

impl<'a> Operators {
    /// Constructor for [Operators].
    pub fn new(addresses: Vec<Address>, weights: Vec<U256>, threshold: U256) -> Self {
        Self {
            addresses,
            weights,
            threshold,
        }
    }

    /// Deserialize [Operators].
    pub fn unpack(input: &'a [u8]) -> Result<Self, AuthWeightedError> {
        match Self::try_from_slice(input) {
            Ok(v) => Ok(v),
            Err(_) => Err(AuthWeightedError::InvalidOperators),
        }
    }

    /// Serialize [Operators].
    pub fn pack(&self) -> Vec<u8> {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        to_vec(&self).unwrap()
    }
}

impl Operators {
    /// Returns threshold.
    pub fn weights(&self) -> &Vec<U256> {
        &self.weights
    }

    /// Returns weight from index.
    pub fn weight_by_index(&self, index: usize) -> &U256 {
        &self.weights[index]
    }

    /// Returns threshold.
    pub fn threshold(&self) -> &U256 {
        &self.threshold
    }

    ///  Returns addresses.
    pub fn addresses(&self) -> &Vec<Address> {
        &self.addresses
    }

    ///  Returns address from index.
    pub fn address_by_index(&self, index: usize) -> &Address {
        &self.addresses[index]
    }
}

impl Operators {
    /// Returns length of [addresses] vector.
    pub fn addresses_len(&self) -> usize {
        self.addresses.len()
    }

    /// Returns weight lenght.
    pub fn weights_len(&self) -> usize {
        self.weights.len()
    }
}
