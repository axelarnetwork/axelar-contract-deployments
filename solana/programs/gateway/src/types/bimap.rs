//! Module for the operator set and epoch biject map.

use bimap::BiBTreeMap;
use borsh::io::Error;
use borsh::io::ErrorKind::{Interrupted, InvalidData};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use thiserror::Error;

use super::address::Address;
use super::hash_new_operator_set;
use crate::error::GatewayError;
use crate::types::u256::U256;

type OperatorsHash = [u8; 32];
type Epoch = U256;

/// Errors that might happen when updating the operator and epocs set.
#[derive(Error, Debug, PartialEq)]
pub enum OperatorsAndEpochsError {
    /// Used for attempts to update the current operator set with existing data.
    #[error("Can't update the operator set with existing data")]
    DuplicateOperators,
}

impl From<OperatorsAndEpochsError> for GatewayError {
    fn from(error: OperatorsAndEpochsError) -> Self {
        use OperatorsAndEpochsError::*;
        msg!("Transfer Operatorship Error: {}", error);
        match error {
            DuplicateOperators => GatewayError::DuplicateOperators,
        }
    }
}

/// Biject map that associates the hash of an operator set with an epoch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OperatorsAndEpochs(bimap::BiBTreeMap<OperatorsHash, Epoch>);

impl OperatorsAndEpochs {
    /// Creates a new `OperatorsAndEpochs` value.
    pub fn new(
        operators_and_weights: impl Iterator<Item = (Address, U256)>,
        threshold: U256,
    ) -> Self {
        let mut instance = Self(BiBTreeMap::new());

        let hash = hash_new_operator_set(operators_and_weights, threshold);
        // safe to unwrap as we are creating a new
        // instance and there are no duplicate entries to error on
        instance.update(hash).unwrap();

        instance
    }

    /// Updates the epoch and operators in the state.
    // TODO: Remove entries from older epochs, as we just need to keep the last 16.
    pub fn update(&mut self, operators_hash: OperatorsHash) -> Result<(), OperatorsAndEpochsError> {
        // We add one so this epoch number matches with the value returned from
        // `Self::current_epoch`
        let new_epoch = self.0.len() as u128 + 1;

        self.0
            .insert_no_overwrite(operators_hash, new_epoch.into())
            .map_err(|_| OperatorsAndEpochsError::DuplicateOperators)
    }

    /// Returns the epoch associated with the given operator hash
    pub fn epoch_for_operator_hash(&self, operators_hash: &OperatorsHash) -> Option<&U256> {
        self.0.get_by_left(operators_hash)
    }

    /// Returns the operator hash associated with the given epoch
    pub fn operator_hash_for_epoch(&self, epoch: &U256) -> Option<&OperatorsHash> {
        self.0.get_by_right(epoch)
    }

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> U256 {
        (self.0.len() as u128).into()
    }
}

impl BorshSerialize for OperatorsAndEpochs {
    #[inline]
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        u16::try_from(self.0.len())
            .map_err(|_| InvalidData)?
            .serialize(writer)?;
        for (hash, epoch) in self.0.iter() {
            epoch.to_le_bytes().serialize(writer)?;
            hash.serialize(writer)?;
        }
        Ok(())
    }
}

impl BorshDeserialize for OperatorsAndEpochs {
    #[inline]
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let mut bimap = BiBTreeMap::new();
        let mut pos = 0;
        let mut epoch_buffer = [0u8; 32];
        let mut hash_buffer = [0u8; 32];
        let len = u16::deserialize_reader(reader)?;
        while pos < len {
            if reader.read(&mut epoch_buffer)? == 0 {
                return Err(Error::new(Interrupted, "Unexpected length of input"));
            };
            let epoch = Epoch::from_le_bytes(epoch_buffer);
            if reader.read(&mut hash_buffer)? == 0 {
                return Err(Error::new(Interrupted, "Unexpected length of input"));
            };
            bimap.insert_no_overwrite(hash_buffer, epoch).map_err(|_| {
                Error::new(
                    InvalidData,
                    "Can't insert duplicated values in the biject map",
                )
            })?;
            pos += 1;
        }
        Ok(OperatorsAndEpochs(bimap))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adding_new_operators() {
        let mut operators_and_epochs = OperatorsAndEpochs::default();
        let operators_hash = [0u8; 32];
        assert!(operators_and_epochs.update(operators_hash).is_ok());
        assert_eq!(operators_and_epochs.current_epoch(), U256::ONE);
    }

    #[test]
    fn test_adding_duplicate_operators() {
        let mut operators_and_epochs = OperatorsAndEpochs::default();
        let operators_hash = [0u8; 32];
        operators_and_epochs.update(operators_hash).unwrap();
        assert_eq!(
            operators_and_epochs.update(operators_hash),
            Err(OperatorsAndEpochsError::DuplicateOperators)
        );
    }

    #[test]
    fn test_epoch_for_existing_operator_hash() {
        let mut operators_and_epochs = OperatorsAndEpochs::default();
        let operators_hash = [0u8; 32];
        operators_and_epochs.update(operators_hash).unwrap();
        assert_eq!(
            operators_and_epochs.epoch_for_operator_hash(&operators_hash),
            Some(&U256::ONE)
        );
    }

    #[test]
    fn test_epoch_for_nonexistent_operator_hash() {
        let operators_and_epochs = OperatorsAndEpochs::default();
        let operators_hash = [0u8; 32];
        assert!(operators_and_epochs
            .epoch_for_operator_hash(&operators_hash)
            .is_none());
    }

    #[test]
    fn borsh_traits() {
        let mut bimap = BiBTreeMap::new();
        bimap.insert([1u8; 32], U256::from(5u8));
        bimap.insert([2u8; 32], U256::from(4u8));
        bimap.insert([3u8; 32], U256::from(3u8));
        bimap.insert([4u8; 32], U256::from(2u8));
        bimap.insert([5u8; 32], U256::ONE);
        let original = OperatorsAndEpochs(bimap);

        let serialized = borsh::to_vec(&original).expect("can serialize Map");
        let deserialized: OperatorsAndEpochs =
            borsh::from_slice(&serialized).expect("can serialize Map");
        assert_eq!(deserialized, original)
    }
}
