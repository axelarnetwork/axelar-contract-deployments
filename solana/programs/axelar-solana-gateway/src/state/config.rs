//! Module for the `GatewayConfig` account type.

use axelar_message_primitives::U256;
use bytemuck::{Pod, Zeroable};
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

use super::BytemuckedPda;

/// Timestamp alias for when the last signer rotation happened
pub type Timestamp = u64;
/// Seconds that need to pass between signer rotations
pub type RotationDelaySecs = u64;
/// Ever-incrementing idx for the signer set
pub type VerifierSetEpoch = U256;

/// Gateway configuration type.
#[derive(Pod, Zeroable, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct GatewayConfig {
    /// current epoch points to the latest signer set hash
    pub current_epoch: VerifierSetEpoch,
    /// how many n epochs do we consider valid
    pub previous_verifier_set_retention: VerifierSetEpoch,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// timestamp tracking of when the previous rotation happened
    pub last_rotation_timestamp: Timestamp,
    /// The gateway operator.
    pub operator: Pubkey,
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// The canonical bump for this account.
    pub bump: u8,
    /// padding for bump
    pub _padding: [u8; 7],
}

impl BytemuckedPda for GatewayConfig {}

impl GatewayConfig {
    /// Returns `true` if the current epoch is still considered valid given the
    /// signer retention policies.
    pub fn is_epoch_valid(&self, epoch: U256) -> Result<bool, GatewayError> {
        let current_epoch = self.current_epoch;
        let elapsed = current_epoch
            .checked_sub(epoch)
            .ok_or(GatewayError::EpochCalculationOverflow)?;

        if elapsed >= self.previous_verifier_set_retention {
            msg!("verifier set is too old");
            return Err(GatewayError::VerifierSetTooOld);
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
