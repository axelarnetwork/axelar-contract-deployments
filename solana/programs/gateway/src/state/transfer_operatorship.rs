//! Transfer Operatorship params account.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::hash::hashv;
use solana_program::msg;
use solana_program::pubkey::Pubkey;
use thiserror::Error;

use crate::error::GatewayError;
use crate::state::discriminator::{Discriminator, TransferOperatorship};
use crate::types::address::Address;
use crate::types::hash_new_operator_set;
use crate::types::u256::U256;

/// Errors that might occur while updating the operators set.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransferOperatorshipError {
    /// The sum of weights was smaller than the required threshold.
    #[error("Insufficient operator weight")]
    InsufficientWeight,
    /// Arithmethic overflow error when summing operator weights.
    #[error("Operator weight sum overflowed the u256 number type")]
    ArithmeticOverflow,
    /// Operators were presented either unordered or more than once.
    #[error("Operators array must be sorted (asc) and unique")]
    UnorderedOrDuplicateOperators,
    /// Thresold was presented as zero, which is an invalid value.
    #[error("Threshold cannot be equal to zero")]
    ZeroThreshold,
    /// The presented operator set was empty.
    #[error("Operator array cannot be empty")]
    EmptyOperators,
}

impl From<TransferOperatorshipError> for GatewayError {
    fn from(error: TransferOperatorshipError) -> Self {
        use TransferOperatorshipError::*;
        msg!("Transfer Operatorship Error: {}", error);

        match error {
            InsufficientWeight => GatewayError::InsufficientOperatorWeight,
            ArithmeticOverflow => GatewayError::ArithmeticOverflow,
            UnorderedOrDuplicateOperators => GatewayError::UnorderedOrDuplicateOperators,
            ZeroThreshold => GatewayError::ZeroThreshold,
            EmptyOperators => GatewayError::EmptyOperators,
        }
    }
}

/// [TransferOperatorshipAccount]; Where instruction parameters are stored.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct TransferOperatorshipAccount {
    /// The account discriminator
    discriminator: Discriminator<TransferOperatorship>,

    /// List of operator addresses and their weights.
    operators_and_weights: Vec<(Address, U256)>,

    /// Desired threshold.
    threshold: U256,
}

impl TransferOperatorshipAccount {
    /// Creates a new value.
    pub fn new(operators_and_weights: Vec<(Address, U256)>, threshold: U256) -> Self {
        Self {
            discriminator: Discriminator::new(),
            operators_and_weights,
            threshold,
        }
    }

    /// Returns list of operators.
    pub fn operators(&self) -> impl Iterator<Item = &Address> {
        self.operators_and_weights.iter().map(|(op, _)| op)
    }

    /// Returns list of weights.
    pub fn weights(&self) -> impl Iterator<Item = &U256> {
        self.operators_and_weights.iter().map(|(_, w)| w)
    }

    /// Returns threshold.
    pub fn threshold(&self) -> U256 {
        self.threshold
    }

    /// Returns the internal hash for this type.
    pub fn hash(&self) -> [u8; 32] {
        hash_new_operator_set(self.operators_and_weights.iter().copied(), self.threshold)
    }

    #[inline]
    /// Returns the PDA, bump and seeds for this account.
    pub fn pda_with_seeds(&self) -> (Pubkey, u8, [u8; 32]) {
        let seeds = hashv(&[b"transfer_operatorship", &self.hash()]).to_bytes();
        let (pubkey, bump) = Pubkey::find_program_address(&[&seeds], &crate::ID);
        (pubkey, bump, seeds)
    }

    /// Returns the PDA and the bump for this account.
    #[inline]
    pub fn pda(&self) -> (Pubkey, u8) {
        let (pda, bump, _seeds) = self.pda_with_seeds();
        (pda, bump)
    }

    /// Validates transfer operatorship data.
    pub fn validate(&self) -> Result<(), TransferOperatorshipError> {
        // Check: non-empty operator list.
        if self.operators_and_weights.is_empty() {
            return Err(TransferOperatorshipError::EmptyOperators);
        }

        // Check: threshold is non-zero.
        if self.threshold == U256::ZERO {
            return Err(TransferOperatorshipError::ZeroThreshold);
        }

        // Check: operator addresses are sorted and are unique.
        if !sorted_and_unique(self.operators()) {
            return Err(TransferOperatorshipError::UnorderedOrDuplicateOperators);
        }

        // Check: sufficient threshold.
        let total_weight: U256 = self
            .weights()
            .try_fold(U256::ZERO, |a, &b| a.checked_add(b))
            .ok_or(TransferOperatorshipError::ArithmeticOverflow)?;
        if total_weight < self.threshold {
            return Err(TransferOperatorshipError::InsufficientWeight);
        }

        Ok(())
    }
}

/// Checks if the given list of byte slices is sorted in ascending order and
/// contains no duplicates.
#[inline]
pub(crate) fn sorted_and_unique<I, T>(addresses: I) -> bool
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
