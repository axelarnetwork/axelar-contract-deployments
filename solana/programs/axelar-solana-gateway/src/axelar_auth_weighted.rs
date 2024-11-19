//! Module for the signer set and epoch biject map.

use std::mem::size_of;

use axelar_message_primitives::U256;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use thiserror::Error;

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
}

/// Timestamp alias for when the last signer rotation happened
pub type Timestamp = u64;
/// Seconds that need to pass between signer rotations
pub type RotationDelaySecs = u64;
/// Ever-incrementing idx for the signer set
pub type SignerSetEpoch = U256;

/// Biject map that associates the hash of an signer set with an epoch.
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
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

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> U256 {
        self.current_epoch
    }

    /// Returns `true` if the current epoch is still considered valid given the
    /// signer retention policies.
    pub fn is_epoch_valid(&self, epoch: U256) -> Result<bool, AxelarAuthWeightedError> {
        let current_epoch = self.current_epoch();
        let elapsed = current_epoch
            .checked_sub(epoch)
            .ok_or(AxelarAuthWeightedError::EpochCalculationOverflow)?;

        if elapsed >= self.previous_signers_retention {
            msg!("signing verifier set is too old");
            return Err(AxelarAuthWeightedError::InvalidSignerSet);
        }
        Ok(true)
    }
}

/// Verifies an ECDSA signature against a given message and public key using the
/// secp256k1 curve.
///
/// Returns `true` if the signature is valid and corresponds to the public key
/// and message; otherwise, returns `false`.
pub fn verify_ecdsa_signature(
    pubkey: &axelar_solana_encoding::types::pubkey::Secp256k1Pubkey,
    signature: &axelar_solana_encoding::types::pubkey::EcdsaRecoverableSignature,
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

/// Verifies an ECDSA signature against a given message and public key using the
/// secp256k1 curve.
///
/// Returns `true` if the signature is valid and corresponds to the public key
/// and message; otherwise, returns `false`.
#[deprecated(note = "Trying to verify Ed25519 signatures on-chain will exhaust the compute budget")]
pub fn verify_eddsa_signature(
    pubkey: &axelar_solana_encoding::types::pubkey::Ed25519Pubkey,
    signature: &axelar_solana_encoding::types::pubkey::Ed25519Signature,
    message: &[u8; 32],
) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let verifying_key = match VerifyingKey::from_bytes(pubkey) {
        Ok(verifying_key) => verifying_key,
        Err(error) => {
            solana_program::msg!("Failed to parse signer public key: {}", error);
            return false;
        }
    };
    let signature = Signature::from_bytes(signature);
    // The implementation of `verify` only returns an atomic variant
    // `InternalError::Verify` in case of verification failure, so we can safely
    // ignore the error value.
    verifying_key.verify(message, &signature).is_ok()
}
