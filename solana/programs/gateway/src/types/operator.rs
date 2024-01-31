//! Operator types.

use borsh::{BorshDeserialize, BorshSerialize};

use super::hash_new_operator_set;
use crate::types::address::Address;
use crate::types::u256::U256;

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

impl Operators {
    /// Constructor for [Operators].
    pub fn new(addresses: Vec<Address>, weights: Vec<U256>, threshold: U256) -> Self {
        Self {
            addresses,
            weights,
            threshold,
        }
    }

    /// Returns the hash for this operator set.
    pub fn hash(&self) -> [u8; 32] {
        let iter = self
            .addresses
            .iter()
            .copied()
            .zip(self.weights.iter().copied());
        hash_new_operator_set(iter, self.threshold)
    }
}

impl Operators {
    /// Returns threshold.
    pub fn weights(&self) -> &[U256] {
        &self.weights
    }

    /// Returns threshold.
    pub fn threshold(&self) -> &U256 {
        &self.threshold
    }

    ///  Returns addresses.
    pub fn addresses(&self) -> &[Address] {
        &self.addresses
    }
}
