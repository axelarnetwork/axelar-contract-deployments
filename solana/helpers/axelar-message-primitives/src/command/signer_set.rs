//! SignerSet types.

use borsh::{BorshDeserialize, BorshSerialize};

use super::{hash_new_signer_set, U256};
use crate::Address;

/// [SignerSet] consist of public keys of signers, weights (bond) and desired
/// threshold.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct SignerSet {
    /// List of addresses; look [Address].
    addresses: Vec<Address>,

    /// List of weights / bond.
    weights: Vec<U256>,

    /// Desired threshold.
    threshold: U256,
}

impl SignerSet {
    /// Constructor for [SignerSet].
    pub fn new(addresses: Vec<Address>, weights: Vec<U256>, threshold: U256) -> Self {
        Self {
            addresses,
            weights,
            threshold,
        }
    }

    /// Returns the hash for this signer set.
    pub fn hash(&self) -> [u8; 32] {
        let iter = self.addresses.iter().zip(self.weights.iter().copied());
        hash_new_signer_set(iter, self.threshold)
    }
}

impl SignerSet {
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
#[inline]
pub fn sorted_and_unique<I, T>(addresses: I) -> bool
where
    I: Iterator<Item = T>,
    T: PartialOrd,
{
    let mut iterator = addresses.peekable();
    while let (Some(a), Some(b)) = (iterator.next(), iterator.peek()) {
        if a >= *b {
            return false;
        }
    }
    true
}

#[test]
fn test_is_sorted_and_unique() {
    assert!(
        sorted_and_unique([[1; 33], [2; 33], [3; 33], [4; 33]].iter()),
        "should return true for sorted and unique elements"
    );
    assert!(
        !sorted_and_unique([[1; 33], [2; 33], [4; 33], [3; 33]].iter()),
        "should return false for unsorted elements"
    );
    assert!(
        !sorted_and_unique([[1; 33], [2; 33], [2; 33], [3; 33]].iter()),
        "should return false for duplicated elements"
    );
}
