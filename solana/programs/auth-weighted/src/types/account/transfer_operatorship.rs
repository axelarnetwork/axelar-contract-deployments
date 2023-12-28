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
    fn valid_threshold(&self) -> bool {
        let total_weight: U256 = self.weights().iter().copied().sum();
        self.threshold() != U256::from(0) && total_weight >= self.threshold()
    }

    /// Checks if the operators data is valid.
    fn valid_operators(&self) -> bool {
        self.operators_len() == 0 || is_sorted_and_unique(self.operators())
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
        if !self.valid_threshold() {
            return Err(AuthWeightedError::InvalidThreshold);
        }

        Ok(())
    }
}

/// Checks if the given list of accounts is sorted in ascending order and
/// contains no duplicates.
pub(super) fn is_sorted_and_unique(addresses: &[Address]) -> bool {
    addresses.windows(2).all(|pair| pair[0] < pair[1])
}

#[test]
fn test_is_sorted_and_unique() {
    // Valid
    let addresses1 = vec![
        Address::new(vec![1, 2, 3]),
        Address::new(vec![2, 3, 4]),
        Address::new(vec![3, 4, 5]),
    ];
    assert!(is_sorted_and_unique(&addresses1));

    // Invalid: Not sorted
    let addresses2 = vec![
        Address::new(vec![3, 4, 5]),
        Address::new(vec![2, 3, 4]),
        Address::new(vec![1, 2, 3]),
    ];
    assert!(!is_sorted_and_unique(&addresses2));

    // Invalid: Duplicates
    let addresses3 = vec![
        Address::new(vec![1, 2, 3]),
        Address::new(vec![2, 3, 4]),
        Address::new(vec![2, 3, 4]),
    ];
    assert!(!is_sorted_and_unique(&addresses3));
}
