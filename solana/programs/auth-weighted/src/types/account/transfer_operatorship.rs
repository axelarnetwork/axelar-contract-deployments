//! Transfer Operatorship params account.

use borsh::{BorshDeserialize, BorshSerialize};

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

impl TransferOperatorshipAccount {
    /// Returns quantity of operators.
    pub fn operators_len(&self) -> usize {
        self.operators.len()
    }

    /// Returns quantity of weights.
    pub fn weights_len(&self) -> usize {
        self.weights.len()
    }

    /// Returns list of operators.
    pub fn operators(&self) -> &[Address] {
        &self.operators
    }

    /// Returns list of weights.
    pub fn weights(&self) -> &[U256] {
        &self.weights
    }

    /// Returns threshold.
    pub fn threshold(&self) -> U256 {
        self.threshold
    }

    /// Verifies if the threshold is valid.
    fn valid_threshold(&self) -> Result<(), AuthWeightedError> {
        if self.threshold == U256::ZERO {
            return Err(AuthWeightedError::InvalidThreshold);
        }
        let total_weight: U256 = self
            .weights()
            .iter()
            .try_fold(U256::ZERO, |a, &b| a.checked_add(b))
            .ok_or(AuthWeightedError::ArithmeticOverflow)?;
        if total_weight < self.threshold {
            Err(AuthWeightedError::InvalidThreshold)
        } else {
            Ok(())
        }
    }

    /// Checks if the operators data is valid.
    fn valid_operators(&self) -> bool {
        self.operators.is_empty() || is_sorted_and_unique(&self.operators)
    }

    /// Validates transfer operatorship data.
    pub fn validate(&self) -> Result<(), AuthWeightedError> {
        // Check: operator addresses are sorted and are unique.
        if !self.valid_operators() {
            return Err(AuthWeightedError::InvalidOperators);
        }

        // Check: weights and operators lenght match.
        if self.weights.len() != self.operators.len() {
            return Err(AuthWeightedError::InvalidWeights);
        }

        // Check: sufficient threshold
        self.valid_threshold()?;

        Ok(())
    }
}

/// Checks if the given list of accounts is sorted in ascending order and
/// contains no duplicates.
pub(super) fn is_sorted_and_unique(addresses: &[Address]) -> bool {
    addresses.windows(2).all(|pair| pair[0] < pair[1])
}

#[test]
fn test_is_sorted_and_unique() -> anyhow::Result<()> {
    /// Creates a 33-byte vector starting with `first_elements` and fill the
    /// rest with zeroes.
    fn prefixed_vector(first_elements: &[u8]) -> Vec<u8> {
        assert!(first_elements.len() < Address::ECDSA_COMPRESSED_PUBKEY_LEN);
        let mut vec = first_elements.to_vec();
        vec.resize(Address::ECDSA_COMPRESSED_PUBKEY_LEN, 0);
        vec
    }

    // Valid
    let addresses1 = vec![
        prefixed_vector(&[1, 2, 3]).try_into()?,
        prefixed_vector(&[2, 3, 4]).try_into()?,
        prefixed_vector(&[3, 4, 5]).try_into()?,
    ];
    assert!(is_sorted_and_unique(&addresses1));

    // Invalid: Not sorted
    let addresses2 = vec![
        prefixed_vector(&[3, 4, 5]).try_into()?,
        prefixed_vector(&[2, 3, 4]).try_into()?,
        prefixed_vector(&[1, 2, 3]).try_into()?,
    ];
    assert!(!is_sorted_and_unique(&addresses2));

    // Invalid: Duplicates
    let addresses3 = vec![
        prefixed_vector(&[1, 2, 3]).try_into()?,
        prefixed_vector(&[2, 3, 4]).try_into()?,
        prefixed_vector(&[2, 3, 4]).try_into()?,
    ];
    assert!(!is_sorted_and_unique(&addresses3));
    Ok(())
}
