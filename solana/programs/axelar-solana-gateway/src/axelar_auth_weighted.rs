//! Module for the signer set and epoch biject map.

use std::mem::size_of;

use axelar_message_primitives::{BnumU256, U256};
use axelar_rkyv_encoding::types::{ArchivedProof, ArchivedVerifierSet, MessageValidationError};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use thiserror::Error;

use crate::state::verifier_set_tracker::VerifierSetTracker;
use crate::{get_verifier_set_tracker_pda, hasher_impl};

/// Errors that might happen when updating the signers and epochs set.
#[derive(Error, Debug)]
pub enum AxelarAuthWeightedError {
    /// Error indicating an underflow occurred during epoch calculation.
    #[error("Epoch calculation resulted in an underflow")]
    EpochCalculationOverflow,

    /// Error indicating the provided signers are invalid.
    #[error("Invalid signer set provided")]
    InvalidSignerSet,

    /// Invalid Weight threshold
    #[error("Invalid Weight threshold")]
    InvalidWeightThreshold,

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
    /// Size of the `AxelarAuthWeighted` struct when serialized.
    pub const SIZE_WHEN_SERIALIZED: usize = {
        size_of::<SignerSetEpoch>()
            + size_of::<SignerSetEpoch>()
            + size_of::<RotationDelaySecs>()
            + size_of::<Timestamp>()
    };

    /// Creates a new `AxelarAuthWeighted` value.
    pub fn new(
        previous_signers_retention: SignerSetEpoch,
        minimum_rotation_delay: RotationDelaySecs,
        current_epoch: SignerSetEpoch,
        current_timestamp: Timestamp,
    ) -> Self {
        Self {
            current_epoch,
            previous_signers_retention,
            minimum_rotation_delay,
            last_rotation_timestamp: current_timestamp,
        }
    }

    /// Ported code from [here](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/10b89fb19a44fe9e51989b618811ddd0e1a595f6/contracts/auth/AxelarAuthWeighted.sol#L30)
    pub fn validate_proof(
        &self,
        message_hash: [u8; 32],
        proof: &ArchivedProof,
        verifier_set_tracker: &VerifierSetTracker,
        domain_separator: &[u8; 32],
    ) -> Result<SignerSetMetadata, AxelarAuthWeightedError> {
        let proof_signer_set_hash = proof.signer_set_hash(hasher_impl(), domain_separator);
        if verifier_set_tracker.verifier_set_hash != proof_signer_set_hash {
            msg!(
                "Provided verifier set tracker does match the derived one. Proof: {}, Registered: {}",
                hex::encode(proof_signer_set_hash),
                hex::encode(verifier_set_tracker.verifier_set_hash),
            );
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }

        let epoch = self.current_epoch();
        let elapsed: BnumU256 = epoch
            .checked_sub(verifier_set_tracker.epoch)
            .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?
            .into();
        if elapsed >= self.previous_signers_retention.into() {
            msg!("signing verifier set is too old");
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }

        validate_proof_for_message(proof, &message_hash)?;

        // this indicates was it the latest known verifier set that signed this proof
        if epoch == verifier_set_tracker.epoch {
            Ok(SignerSetMetadata::Latest)
        } else {
            Ok(SignerSetMetadata::ValidOld)
        }
    }

    /// Ported code from [here](https://github.com/axelarnetwork/cgp-spec/blob/c3010b9187ad9022dbba398525cf4ec35b75e7ae/solidity/contracts/auth/AxelarAuthWeighted.sol#L61)
    pub fn rotate_signers(
        &mut self,
        new_verifier_set: &ArchivedVerifierSet,
    ) -> Result<VerifierSetTracker, AxelarAuthWeightedError> {
        // signers must be sorted binary or alphabetically in lower case
        if new_verifier_set.is_empty() {
            msg!("No signers in the new set");
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }

        if !matches!(new_verifier_set.sufficient_weight(), Some(true)) {
            msg!("insufficient weight for the new verifier set");
            return Err(AxelarAuthWeightedError::InvalidWeightThreshold);
        }

        let new_verifier_set_hash = new_verifier_set.hash(hasher_impl());
        let (_, bump) = get_verifier_set_tracker_pda(&crate::id(), new_verifier_set_hash);

        self.current_epoch = self
            .current_epoch
            .checked_add(U256::ONE)
            .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?;
        Ok(VerifierSetTracker {
            bump,
            epoch: self.current_epoch,
            verifier_set_hash: new_verifier_set_hash,
        })
    }

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> U256 {
        self.current_epoch
    }

    /// Returns `true` if the current epoch is still considered valid given the
    /// signer retention policies.
    pub fn is_epoch_valid(&self, epoch: U256) -> Result<bool, AxelarAuthWeightedError> {
        let earliest_valid_epoch = self
            .current_epoch
            .checked_sub(self.previous_signers_retention)
            .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?;
        Ok(epoch >= earliest_valid_epoch)
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
        let current_epoch = SignerSetEpoch::deserialize_reader(reader)?;
        let previous_signers_retention = SignerSetEpoch::deserialize_reader(reader)?;
        let last_rotation_timestamp = Timestamp::deserialize_reader(reader)?;
        let minimum_rotation_delay = RotationDelaySecs::deserialize_reader(reader)?;

        Ok(AxelarAuthWeighted {
            current_epoch,
            previous_signers_retention,
            minimum_rotation_delay,
            last_rotation_timestamp,
        })
    }
}

#[cfg(test)]
mod tests {

    use axelar_rkyv_encoding::types::{PublicKey, Signature};
    use solana_sdk::pubkey::Pubkey;

    use super::*;
    use crate::state::GatewayConfig;

    const DOMAIN_SEPARATOR: [u8; 32] = [77u8; 32];
    const DEFAULT_PREVIOUS_SIGNERS_RETENTION: U256 = U256::from_u64(4);
    const DEFAULT_MINIMUM_ROTATION_DELAY: RotationDelaySecs = 42;
    const DEFAULT_TIMESTAMP: Timestamp = 88;

    #[test]
    fn test_initial_signer_set_count_as_first_epoch() {
        let current_timestamp = 88;
        let aw = AxelarAuthWeighted::new(
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
            U256::ONE,
            current_timestamp,
        );
        assert_eq!(aw.current_epoch(), U256::ONE);
        assert_eq!(
            aw.previous_signers_retention,
            DEFAULT_PREVIOUS_SIGNERS_RETENTION
        );
        assert_eq!(aw.minimum_rotation_delay, DEFAULT_MINIMUM_ROTATION_DELAY);
        assert_eq!(aw.last_rotation_timestamp, current_timestamp);
    }

    #[test]
    fn serialization_roundtrip() {
        let bump = 255;
        let aw = AxelarAuthWeighted::new(
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
            U256::ONE,
            DEFAULT_TIMESTAMP,
        );
        let config = GatewayConfig::new(bump, aw, Pubkey::new_unique(), DOMAIN_SEPARATOR);
        let serialized = borsh::to_vec(&config).unwrap();
        let deserialized: GatewayConfig = borsh::from_slice(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn serialization_min_signer_set_auth_weighted_matches_expected_len() {
        let aw = AxelarAuthWeighted::new(
            DEFAULT_PREVIOUS_SIGNERS_RETENTION,
            DEFAULT_MINIMUM_ROTATION_DELAY,
            U256::ONE,
            DEFAULT_TIMESTAMP,
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
