//! Module for the signer set and epoch biject map.

use std::mem::size_of;

use axelar_message_primitives::command::{
    hash_new_signer_set, sorted_and_unique, Proof, ProofError, RotateSignersCommand, U256,
};
use axelar_message_primitives::Address;
use bimap::BiBTreeMap;
use borsh::io::Error;
use borsh::io::ErrorKind::{Interrupted, InvalidData};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use thiserror::Error;

type SignerSetHash = [u8; 32];
type Epoch = U256;

/// Errors that might happen when updating the signers and epochs set.
#[derive(Error, Debug, PartialEq)]
pub enum AxelarAuthWeightedError {
    /// Error indicating an attempt to update the current signer set with data
    /// that already exists.
    #[error("Can't update the signer set with existing data")]
    DuplicateSignerSet,

    /// Error indicating the specified epoch was not found.
    #[error("Epoch not found")]
    EpochNotFound,

    /// Error indicating an underflow occurred during epoch calculation.
    #[error("Epoch calculation resulted in an underflow")]
    EpochCalculationOverflow,

    /// Error indicating an overflow occurred during weight calculation.
    #[error("Weight calculation resulted in an overflow")]
    WeightCalculationOverflow,

    /// Error indicating the provided signers are invalid.
    #[error("Invalid signer set provided")]
    InvalidSignerSet,

    /// Invalid Weight length
    #[error("Invalid Weight length")]
    InvalidWeightLength,

    /// Invalid Weight threshold
    #[error("Invalid Weight threshold")]
    InvalidWeightThreshold,

    /// Error indicating the sum of signature weights is below the required
    /// threshold.
    #[error("The sum of signature weights is below the required threshold")]
    LowSignaturesWeight,

    /// Error indicating the signers are malformed.
    #[error("Malformed signers provided")]
    MalformedSigners,

    /// Error wrapping a `Secp256k1RecoverError` from the
    /// `solana_program::secp256k1_recover` module.
    #[error(transparent)]
    Secp256k1RecoverError(#[from] solana_program::secp256k1_recover::Secp256k1RecoverError),

    /// Error wrapping a `ProofError` from the
    /// `axelar_message_primitives::command` module.
    #[error(transparent)]
    ProofError(#[from] ProofError),
}

/// Biject map that associates the hash of an signer set with an epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxelarAuthWeighted {
    // TODO we could replace this with something that has a known static size, like something from the heapless crate - https://docs.rs/heapless/latest/heapless/struct.IndexMap.html
    map: bimap::BiBTreeMap<SignerSetHash, Epoch>,
    current_epoch: Epoch,
}

/// Derived metadata information about the signer set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignerSetMetadata {
    /// Indicates theat the signer set is the most recent-known one.
    Latest,
    /// Indicates that the signer set is a valid but is not the most
    /// recent-known one.
    ValidOld,
}

impl AxelarAuthWeighted {
    const OLD_KEY_RETENTION: u8 = 4;
    /// Size of the `AxelarAuthWeighted` struct when serialized.
    pub const SIZE_WHEN_SERIALIZED: usize = {
        // len of map + len of current_epoch + len of 4 signer_sets_hash + len of 4
        // epochs
        size_of::<u8>()
            + size_of::<U256>()
            + (size_of::<SignerSetHash>() * Self::OLD_KEY_RETENTION as usize)
            + (size_of::<Epoch>() * Self::OLD_KEY_RETENTION as usize)
    };

    /// Creates a new `AxelarAuthWeighted` value.
    pub fn new<'a>(
        signer_set_and_weights: impl Iterator<Item = (&'a Address, U256)>,
        threshold: U256,
    ) -> Self {
        let mut instance = Self {
            map: BiBTreeMap::new(),
            current_epoch: U256::ZERO,
        };

        // TODO this does not mach AxelarAuthWeighted contrsuctor from Solidity!

        let hash = hash_new_signer_set(signer_set_and_weights, threshold);
        // safe to unwrap as we are creating a new
        // instance and there are no duplicate entries to error on
        instance.update_latest_signer_set(hash).unwrap();

        instance
    }

    /// Ported code from [here](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/10b89fb19a44fe9e51989b618811ddd0e1a595f6/contracts/auth/AxelarAuthWeighted.sol#L30)
    pub fn validate_proof(
        &self,
        message_hash: [u8; 32],
        proof: &Proof,
    ) -> Result<SignerSetMetadata, AxelarAuthWeightedError> {
        let signer_set_hash = proof.signer_set_hash();
        let signer_set_epoch = self
            .epoch_for_signer_set_hash(&signer_set_hash)
            .ok_or(AxelarAuthWeightedError::EpochNotFound)?;
        let epoch = self.current_epoch();
        if epoch
            .checked_sub(*signer_set_epoch)
            .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?
            >= U256::from(Self::OLD_KEY_RETENTION)
        {
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }

        proof.validate_signatures(&message_hash)?;

        if epoch == *signer_set_epoch {
            Ok(SignerSetMetadata::Latest)
        } else {
            Ok(SignerSetMetadata::ValidOld)
        }
    }

    /// Ported code from [here](https://github.com/axelarnetwork/cgp-spec/blob/c3010b9187ad9022dbba398525cf4ec35b75e7ae/solidity/contracts/auth/AxelarAuthWeighted.sol#L61)
    pub fn rotate_signers(
        &mut self,
        new_command: &RotateSignersCommand,
    ) -> Result<(), AxelarAuthWeightedError> {
        // signers must be sorted binary or alphabetically in lower case
        if new_command.signer_set.is_empty() || !sorted_and_unique(new_command.signer_set.iter()) {
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }

        if new_command.weights.len() != new_command.signer_set.len() {
            return Err(AxelarAuthWeightedError::InvalidWeightLength);
        }

        let total_weight: U256 = new_command
            .weights
            .iter()
            .try_fold(U256::ZERO, |a, &b| a.checked_add(b.into()))
            .ok_or(AxelarAuthWeightedError::WeightCalculationOverflow)?;

        if total_weight == U256::ZERO || total_weight < new_command.quorum.into() {
            return Err(AxelarAuthWeightedError::InvalidWeightThreshold);
        }

        let new_signer_set_hash = hash_new_signer_set(
            new_command
                .signer_set
                .iter()
                .zip(new_command.weights.iter()),
            new_command.quorum.into(),
        );
        if self
            .epoch_for_signer_set_hash(&new_signer_set_hash)
            .is_some()
        {
            return Err(AxelarAuthWeightedError::DuplicateSignerSet);
        }
        self.update_latest_signer_set(new_signer_set_hash)?;
        Ok(())
    }

    /// Updates the epoch and signers in the state.
    fn update_latest_signer_set(
        &mut self,
        signer_set_hash: SignerSetHash,
    ) -> Result<(), AxelarAuthWeightedError> {
        // We add one so this epoch number matches with the value returned from
        // `Self::current_epoch`
        self.current_epoch = self
            .current_epoch
            .checked_add(U256::ONE)
            .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?;

        self.map
            .insert_no_overwrite(signer_set_hash, self.current_epoch)
            .map_err(|_| AxelarAuthWeightedError::DuplicateSignerSet)?;

        // Remove a single old entry
        if self.map.len() > Self::OLD_KEY_RETENTION as usize {
            // Safe to unwrap as we are removing the oldest entry and we know
            // OLD_KEY_RETENTION is > 0
            let oldest_epoch = self
                .current_epoch
                .checked_sub(U256::from(Self::OLD_KEY_RETENTION))
                .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?;
            msg!(&format!("removing {}", oldest_epoch));
            self.map.remove_by_right(&oldest_epoch);
        }

        Ok(())
    }

    /// Returns the epoch associated with the given signer set hash
    pub fn epoch_for_signer_set_hash(&self, signer_set_hash: &SignerSetHash) -> Option<&U256> {
        self.map.get_by_left(signer_set_hash)
    }

    /// Returns the signer set hash associated with the given epoch
    pub fn signer_set_hash_for_epoch(&self, epoch: &U256) -> Option<&SignerSetHash> {
        self.map.get_by_right(epoch)
    }

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> U256 {
        self.current_epoch
    }

    /// Get read only access to the underlying signer set map
    pub fn signer_sets(&self) -> &bimap::BiBTreeMap<SignerSetHash, Epoch> {
        &self.map
    }
}

impl BorshSerialize for AxelarAuthWeighted {
    /// The serialization format is as follows:
    /// [u8: map length]
    /// [u256: current epoch]
    /// [[epoch: hash], ..n times Self::OLD_KEY_RETENTION  ] -- empty data
    /// filled with 0s
    #[inline]
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        u8::try_from(self.map.len())
            .map_err(|_| InvalidData)?
            .serialize(writer)?;
        self.current_epoch.serialize(writer)?;
        for (hash, epoch) in self.map.iter() {
            epoch.to_le_bytes().serialize(writer)?;
            hash.serialize(writer)?;
        }
        // fill the rest of the data with empty bytes
        let items_to_fill = Self::OLD_KEY_RETENTION - self.map.len() as u8;
        for _ in 0..items_to_fill {
            [0u8; 32].serialize(writer)?;
            [0u8; 32].serialize(writer)?;
        }

        Ok(())
    }
}

impl BorshDeserialize for AxelarAuthWeighted {
    #[inline]
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let mut bimap = BiBTreeMap::new();
        let mut pos = 0;
        let mut epoch_buffer = [0u8; 32];
        let mut hash_buffer = [0u8; 32];
        let map_len = u8::deserialize_reader(reader)?;
        let current_epoch = U256::deserialize_reader(reader)?;
        while pos < map_len {
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

        // We need to consume the empty data otherwise borsh will fail if there's unread
        // data in the buffer
        let empty_items_to_consume = Self::OLD_KEY_RETENTION - map_len;
        for _ in 0..empty_items_to_consume {
            // ignore the returned length t hat we read as we are just consuming the data
            let _ = reader.read(&mut epoch_buffer)?;
            let _ = reader.read(&mut hash_buffer)?;
        }

        Ok(AxelarAuthWeighted {
            map: bimap,
            current_epoch,
        })
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::pubkey::Pubkey;

    use super::*;
    use crate::state::GatewayConfig;

    #[test]
    fn test_initial_signer_set_count_as_first_epoch() {
        let aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        assert_eq!(aw.current_epoch(), U256::ONE);
    }

    #[test]
    fn test_adding_new_signer_set() {
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_set_hash = [0u8; 32];
        assert!(aw.update_latest_signer_set(signer_set_hash).is_ok());
        assert_eq!(aw.current_epoch(), U256::from(2_u8));
    }

    #[test]
    fn test_adding_duplicate_signer_set() {
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_set_hash = [0u8; 32];
        aw.update_latest_signer_set(signer_set_hash).unwrap();
        assert_eq!(
            aw.update_latest_signer_set(signer_set_hash),
            Err(AxelarAuthWeightedError::DuplicateSignerSet)
        );
    }

    #[test]
    fn test_epoch_for_existing_signer_set_hash() {
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_set_hash = [0u8; 32];
        aw.update_latest_signer_set(signer_set_hash).unwrap();
        assert_eq!(
            aw.epoch_for_signer_set_hash(&signer_set_hash),
            Some(&U256::from(2_u8))
        );
        assert_eq!(aw.current_epoch(), U256::from(2_u8));
    }

    #[test]
    fn test_epoch_for_nonexistent_signer_set_hash() {
        let aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_sets_hash = [0u8; 32];
        assert!(aw.epoch_for_signer_set_hash(&signer_sets_hash).is_none());
    }

    #[test]
    fn borsh_traits() {
        let mut bimap = BiBTreeMap::new();
        bimap.insert([2u8; 32], U256::from(4u8));
        bimap.insert([3u8; 32], U256::from(3u8));
        bimap.insert([4u8; 32], U256::from(2u8));
        bimap.insert([5u8; 32], U256::ONE);
        let original = AxelarAuthWeighted {
            map: bimap,
            current_epoch: U256::from_le_bytes([u8::MAX; 32]),
        };

        let serialized = borsh::to_vec(&original).expect("can serialize Map");
        let deserialized: AxelarAuthWeighted =
            borsh::from_slice(&serialized).expect("can serialize Map");
        assert_eq!(deserialized, original)
    }

    #[test]
    fn serialization_roundtrip() {
        let bump = 255;
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        aw.update_latest_signer_set([1u8; 32]).unwrap();
        aw.update_latest_signer_set([2u8; 32]).unwrap();
        aw.update_latest_signer_set([3u8; 32]).unwrap();
        let config = GatewayConfig::new(bump, aw, Pubkey::new_unique());
        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized: GatewayConfig = borsh::from_slice(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn only_keeping_the_last_16_entries() {
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_set_to_insert = AxelarAuthWeighted::OLD_KEY_RETENTION * 2;
        for i in 0..signer_set_to_insert {
            let signer_set_hash = [i; 32];
            aw.update_latest_signer_set(signer_set_hash).unwrap();
            assert_eq!(
                aw.map.len() as u8,
                (i
                    // when we init, we start at 1
                    + 1
                    // we start iterating from 0
                    + 1)
                .min(AxelarAuthWeighted::OLD_KEY_RETENTION),
                "always stays at 16 or less entries"
            );
        }
        assert_eq!(aw.current_epoch(), U256::from(signer_set_to_insert + 1));
        assert_eq!(aw.map.len(), AxelarAuthWeighted::OLD_KEY_RETENTION as usize);
    }

    #[test]
    fn serialization_roundtrip_max_signer_set_gateway() {
        let bump = 255;
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_set_to_insert = AxelarAuthWeighted::OLD_KEY_RETENTION * 2;
        for i in 0..signer_set_to_insert {
            let signer_set_hash = [i; 32];
            aw.update_latest_signer_set(signer_set_hash).unwrap();
        }
        let config = GatewayConfig::new(bump, aw, Pubkey::new_unique());
        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized: GatewayConfig = borsh::from_slice(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn serialization_max_signer_set_auth_weighted_matches_expected_len() {
        let mut aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let signer_set_to_insert = AxelarAuthWeighted::OLD_KEY_RETENTION * 2;
        for i in 0..signer_set_to_insert {
            let signer_set_hash = [i; 32];
            aw.update_latest_signer_set(signer_set_hash).unwrap();
        }
        let serialized = borsh::to_vec(&aw).unwrap();
        assert_eq!(serialized.len(), AxelarAuthWeighted::SIZE_WHEN_SERIALIZED);
    }

    #[test]
    fn serialization_min_signer_set_auth_weighted_matches_expected_len() {
        let aw = AxelarAuthWeighted::new([].into_iter(), U256::ZERO);
        let serialized = borsh::to_vec(&aw).unwrap();
        assert_eq!(serialized.len(), AxelarAuthWeighted::SIZE_WHEN_SERIALIZED);
    }
}
