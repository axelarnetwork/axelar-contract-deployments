//! Module for the signer set and epoch biject map.

use std::mem::size_of;

use axelar_message_primitives::command::{ProofError, U256};
use axelar_rkyv_encoding::types::{ArchivedProof, ArchivedVerifierSet, MessageValidationError};
use bimap::BiBTreeMap;
use borsh::io::Error;
use borsh::io::ErrorKind::{Interrupted, InvalidData};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use thiserror::Error;

use crate::hasher_impl;

type SignerSetHash = [u8; 32];

/// Errors that might happen when updating the signers and epochs set.
#[derive(Error, Debug)]
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

    /// Error wrapping a `MessageValidationError` from the
    /// `axelar_rkyv_encoding` crate.
    #[error(transparent)]
    MessageValidationError(#[from] MessageValidationError),
}

/// Timestamp alias for when the last signer rotation happened
pub type Timestamp = u64;
/// Seconds that need to pass between signer rotations
pub type RotationDelaySecs = u64;
/// Ever-incrementing idx for the signer set
pub type SignerSetEpoch = U256;

/// Biject map that associates the hash of an signer set with an epoch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AxelarAuthWeighted {
    map: bimap::BiBTreeMap<SignerSetHash, SignerSetEpoch>,
    /// current epoch points to the latest signer set hash
    pub current_epoch: SignerSetEpoch,
    /// how many n epochs do we consider valid
    pub previous_signers_retention: SignerSetEpoch,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// timestamp tracking of when the previous rotation happened
    pub last_rotation_timestamp: Timestamp,
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
    // todo: remove this variable and use `self.minimum_rotation_delay` once we no
    // longer store the signer set hashes in a hashmap but rather use PDAs
    const OLD_KEY_RETENTION: u8 = 4;
    /// Size of the `AxelarAuthWeighted` struct when serialized.
    pub const SIZE_WHEN_SERIALIZED: usize = {
        size_of::<u8>()
            + size_of::<U256>()
            + (size_of::<SignerSetHash>() * Self::OLD_KEY_RETENTION as usize)
            + (size_of::<SignerSetEpoch>() * Self::OLD_KEY_RETENTION as usize)
            + size_of::<SignerSetEpoch>()
            + size_of::<RotationDelaySecs>()
            + size_of::<Timestamp>()
    };

    /// Creates a new `AxelarAuthWeighted` value.
    pub fn new<'a>(
        verifier_sets: impl Iterator<Item = &'a ArchivedVerifierSet>,
        previous_signers_retention: SignerSetEpoch,
        minimum_rotation_delay: RotationDelaySecs,
    ) -> Self {
        let mut instance = Self {
            map: BiBTreeMap::new(),
            current_epoch: U256::ZERO,
            previous_signers_retention,
            minimum_rotation_delay,
            last_rotation_timestamp: 0, // this will be updated in the first rotation
        };

        for item in verifier_sets {
            let signer_set_hash = item.hash(hasher_impl());
            // safe to unwrap as we are creating a new
            // instance and there are no duplicate entries to error on
            instance.update_latest_signer_set(signer_set_hash).unwrap();
        }

        instance
    }

    /// Ported code from [here](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/10b89fb19a44fe9e51989b618811ddd0e1a595f6/contracts/auth/AxelarAuthWeighted.sol#L30)
    pub fn validate_proof(
        &self,
        message_hash: [u8; 32],
        proof: &ArchivedProof,
    ) -> Result<SignerSetMetadata, AxelarAuthWeightedError> {
        let signer_set_hash = proof.signer_set_hash(hasher_impl());
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

        validate_proof_for_message(proof, &message_hash)?;

        if epoch == *signer_set_epoch {
            Ok(SignerSetMetadata::Latest)
        } else {
            Ok(SignerSetMetadata::ValidOld)
        }
    }

    /// Ported code from [here](https://github.com/axelarnetwork/cgp-spec/blob/c3010b9187ad9022dbba398525cf4ec35b75e7ae/solidity/contracts/auth/AxelarAuthWeighted.sol#L61)
    pub fn rotate_signers(
        &mut self,
        new_verifier_set: &ArchivedVerifierSet,
    ) -> Result<(), AxelarAuthWeightedError> {
        // signers must be sorted binary or alphabetically in lower case
        if new_verifier_set.is_empty() {
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }

        if !matches!(new_verifier_set.sufficient_weight(), Some(true)) {
            return Err(AxelarAuthWeightedError::InvalidWeightThreshold);
        }

        let new_verifier_set_hash = new_verifier_set.hash(hasher_impl());
        if self
            .epoch_for_signer_set_hash(&new_verifier_set_hash)
            .is_some()
        {
            return Err(AxelarAuthWeightedError::DuplicateSignerSet);
        }
        self.update_latest_signer_set(new_verifier_set_hash)?;
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
    pub fn signer_sets(&self) -> &bimap::BiBTreeMap<SignerSetHash, SignerSetEpoch> {
        &self.map
    }
}

fn validate_proof_for_message(
    proof: &ArchivedProof,
    message_hash: &[u8; 32],
) -> Result<(), AxelarAuthWeightedError> {
    Ok(proof.validate_for_message_custom(
        message_hash,
        verify_ecdsa_signature,
        verify_eddsa_signature,
    )?)
}

fn verify_ecdsa_signature(
    pubkey: &axelar_rkyv_encoding::types::Secp256k1Pubkey,
    signature: &axelar_rkyv_encoding::types::EcdsaRecoverableSignature,
    message: &[u8; 32],
) -> bool {
    // The recovery bit in the signature's bytes is placed at the end, as per the
    // 'multisig-prover' contract by Axelar. Unwrap: we know the 'signature'
    // slice exact size, and it isn't empty.
    let (signature, recovery_id) = match signature {
        [first_64 @ .., recovery_id] => (first_64, recovery_id),
    };

    // Transform from Ethereum recovery_id (27, 28) to a range accepted by
    // secp256k1_recover (0, 1, 2, 3)
    let recovery_id = if *recovery_id >= 27 {
        recovery_id - 27
    } else {
        *recovery_id
    };

    // This is results in a Solana syscall.
    let secp256k1_recover =
        solana_program::secp256k1_recover::secp256k1_recover(message, recovery_id, signature);
    let Ok(recovered_uncompressed_pubkey) = secp256k1_recover else {
        msg!("Failed to recover ECDSA signature");
        return false;
    };

    // unwrap: provided pukey is guaranteed to be secp256k1 key
    let pubkey = libsecp256k1::PublicKey::parse_compressed(pubkey)
        .unwrap()
        .serialize();

    // we drop the const prefix byte that indicates that this is an uncompressed
    // pubkey
    let full_pubkey = match pubkey {
        [_tag, pubkey @ ..] => pubkey,
    };
    recovered_uncompressed_pubkey.to_bytes() == full_pubkey
}

fn verify_eddsa_signature(
    _pubkey: &axelar_rkyv_encoding::types::Ed25519Pubkey,
    _signature: &axelar_rkyv_encoding::types::Ed25519Signature,
    _message: &[u8; 32],
) -> bool {
    unimplemented!("eddsa signature verification is unimplemented")
}

impl BorshSerialize for AxelarAuthWeighted {
    /// The serialization format is as follows:
    /// - [u8: map length]
    /// - [[epoch: hash], ..n times Self::OLD_KEY_RETENTION  ] -- empty data
    ///   filled with 0s
    /// - [u256: current epoch]
    /// - [u128: old key retention]
    /// - [u128: last timestamp]
    /// - [u128: rotation delay in secs]
    #[inline]
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        // map related tasks
        {
            u8::try_from(self.map.len())
                .map_err(|_| InvalidData)?
                .serialize(writer)?;
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
        }

        self.current_epoch.serialize(writer)?;
        self.previous_signers_retention.serialize(writer)?;
        self.last_rotation_timestamp.serialize(writer)?;
        self.minimum_rotation_delay.serialize(writer)?;

        Ok(())
    }
}

impl BorshDeserialize for AxelarAuthWeighted {
    #[inline]
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        // map related tasks
        let bimap = {
            let mut bimap = BiBTreeMap::new();
            let mut pos = 0;
            let mut epoch_buffer = [0u8; 32];
            let mut hash_buffer = [0u8; 32];
            let map_len = u8::deserialize_reader(reader)?;
            while pos < map_len {
                if reader.read(&mut epoch_buffer)? == 0 {
                    return Err(Error::new(Interrupted, "Unexpected length of input"));
                };
                let epoch = SignerSetEpoch::from_le_bytes(epoch_buffer);
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
            bimap
        };

        let current_epoch = SignerSetEpoch::deserialize_reader(reader)?;
        let previous_signers_retention = SignerSetEpoch::deserialize_reader(reader)?;
        let last_rotation_timestamp = Timestamp::deserialize_reader(reader)?;
        let minimum_rotation_delay = RotationDelaySecs::deserialize_reader(reader)?;

        Ok(AxelarAuthWeighted {
            map: bimap,
            current_epoch,
            previous_signers_retention,
            minimum_rotation_delay,
            last_rotation_timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use axelar_rkyv_encoding::test_fixtures::random_valid_verifier_set;
    use axelar_rkyv_encoding::types::{PublicKey, Signature};
    use solana_sdk::pubkey::Pubkey;

    use super::*;
    use crate::instructions::VerifierSetWraper;
    use crate::state::GatewayConfig;

    const DOMAIN_SEPARATOR: [u8; 32] = [0u8; 32];

    const DEFAULT_PREVIOUS_SIGNERS_RETENTION: U256 = U256::from_u64(4);

    const DEFAULT_MINIMUM_ROTATION_DELAY: RotationDelaySecs = 0;

    fn random_verifier_set() -> VerifierSetWraper {
        VerifierSetWraper::new_from_verifier_set(random_valid_verifier_set()).unwrap()
    }

    #[test]
    fn test_initial_signer_set_count_as_first_epoch() {
        let aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
        assert_eq!(aw.current_epoch(), U256::ONE);
    }

    #[test]
    fn test_adding_new_signer_set() {
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
        let signer_set_hash = [0u8; 32];
        assert!(aw.update_latest_signer_set(signer_set_hash).is_ok());
        assert_eq!(aw.current_epoch(), U256::from(2_u8));
    }

    #[test]
    fn test_adding_duplicate_signer_set() {
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
        let signer_set_hash = [0u8; 32];
        aw.update_latest_signer_set(signer_set_hash).unwrap();
        assert!(matches!(
            aw.update_latest_signer_set(signer_set_hash).unwrap_err(),
            AxelarAuthWeightedError::DuplicateSignerSet
        ));
    }

    #[test]
    fn test_epoch_for_existing_signer_set_hash() {
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
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
        let aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
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
            previous_signers_retention: DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            minimum_rotation_delay: DEFAULT_MINIMUM_ROTATION_DELAY,
            last_rotation_timestamp: 555,
        };

        let serialized = borsh::to_vec(&original).expect("can serialize Map");
        let deserialized: AxelarAuthWeighted =
            borsh::from_slice(&serialized).expect("can serialize Map");
        assert_eq!(deserialized, original)
    }

    #[test]
    fn serialization_roundtrip() {
        let bump = 255;
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
        aw.update_latest_signer_set([1u8; 32]).unwrap();
        aw.update_latest_signer_set([2u8; 32]).unwrap();
        aw.update_latest_signer_set([3u8; 32]).unwrap();
        let config = GatewayConfig::new(bump, aw, Pubkey::new_unique(), DOMAIN_SEPARATOR);
        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized: GatewayConfig = borsh::from_slice(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn only_keeping_the_last_16_entries() {
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
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
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
        let signer_set_to_insert = AxelarAuthWeighted::OLD_KEY_RETENTION * 2;
        for i in 0..signer_set_to_insert {
            let signer_set_hash = [i; 32];
            aw.update_latest_signer_set(signer_set_hash).unwrap();
        }
        let config = GatewayConfig::new(bump, aw, Pubkey::new_unique(), DOMAIN_SEPARATOR);
        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized: GatewayConfig = borsh::from_slice(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn serialization_max_signer_set_auth_weighted_matches_expected_len() {
        let mut aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
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
        let aw = AxelarAuthWeighted::new(
            [random_verifier_set()].iter().map(|x| x.parse().unwrap()),
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
        );
        let serialized = borsh::to_vec(&aw).unwrap();
        assert_eq!(serialized.len(), AxelarAuthWeighted::SIZE_WHEN_SERIALIZED);
    }

    #[test]
    fn can_verify_signatures_with_ecrecover_recovery_id() {
        let (keypair, pubkey) = axelar_rkyv_encoding::test_fixtures::random_ecdsa_keypair();
        let message_hash = [42; 32];
        let signature = keypair.sign(&message_hash);
        let Signature::EcdsaRecoverable(mut signature) = signature else {
            panic!("unexpected signature type");
        };
        signature[64] += 27;
        let PublicKey::Secp256k1(pubkey) = pubkey else {
            panic!("unexpected pubkey type");
        };

        let is_valid = verify_ecdsa_signature(&pubkey, &signature, &message_hash);
        assert!(is_valid);
    }

    #[test]
    fn can_verify_signatures_with_standard_recovery_id() {
        let (keypair, pubkey) = axelar_rkyv_encoding::test_fixtures::random_ecdsa_keypair();
        let message_hash = [42; 32];
        let signature = keypair.sign(&message_hash);
        let Signature::EcdsaRecoverable(signature) = signature else {
            panic!("unexpected signature type");
        };
        assert!((0_u8..=3_u8).contains(&signature[64]));
        let PublicKey::Secp256k1(pubkey) = pubkey else {
            panic!("unexpected pubkey type");
        };

        let is_valid = verify_ecdsa_signature(&pubkey, &signature, &message_hash);
        assert!(is_valid);
    }
}
