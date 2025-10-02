//! Multi-step signature verification.

use axelar_solana_encoding::hasher::{Hasher, SolanaSyscallHasher};
use axelar_solana_encoding::types::execute_data::SigningVerifierSetInfo;
use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
use axelar_solana_encoding::types::verifier_set::VerifierSetLeaf;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView;
use bytemuck::{Pod, Zeroable};
use program_utils::pda::BytemuckedPda;

use crate::error::GatewayError;

use super::verifier_set_tracker::VerifierSetHash;

/// Controls the signature verification session for a given payload.
#[repr(C)]
#[derive(Zeroable, Pod, Clone, Default, Copy, PartialEq, Eq, Debug)]
pub struct SignatureVerification {
    /// Accumulated signer threshold required to validate the payload.
    ///
    /// Is incremented on each successful verification.
    ///
    /// Set to [`u128::MAX`] once the accumulated threshold is greater than or
    /// equal the current verifier set threshold.
    pub accumulated_threshold: u128,

    /// A bit field used to track which signatures have been verified.
    ///
    /// Initially, all bits are set to zero. When a signature is verified, its
    /// corresponding bit is flipped to one. This prevents the same signature
    /// from being verified more than once, avoiding deliberate attempts to
    /// decrement the remaining threshold.
    ///
    /// Currently supports 256 slots. If the signer set maximum size needs to be
    /// increased in the future, this value must change to make roof for
    /// them.
    pub signature_slots: [u8; 32],

    /// Upon the first successful signature validation, we set the hash of the
    /// signing verifier set.
    /// This data is later used when rotating signers to figure out which
    /// verifier set was the one that actually performed the validation.
    pub signing_verifier_set_hash: VerifierSetHash,
}

impl BytemuckedPda for SignatureVerification {}

impl SignatureVerification {
    /// Returns `true` if a sufficient number of signatures have been verified.
    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.accumulated_threshold == u128::MAX
    }

    /// Fully process a submitted signature.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError`] if any of the following conditions occur:
    /// * [`GatewayError::InvalidMerkleProof`] if the Merkle proof bytes are invalid or malformed
    /// * [`GatewayError::SlotAlreadyProcessed`] if the verifier's slot has already been processed
    /// * [`GatewayError::InvalidMerkleProof`] if the Merkle proof verification fails
    /// * [`GatewayError::InvalidSignature`] if the digital signature is invalid
    /// * Additional errors may occur during slot marking or verifier set initialization
    pub fn process_signature(
        &mut self,
        verifier_info: &SigningVerifierSetInfo,
        verifier_set_merkle_root: &[u8; 32],
        payload_merkle_root: &[u8; 32],
    ) -> Result<(), GatewayError> {
        let merkle_proof =
            rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&verifier_info.merkle_proof)
                .map_err(|_err| GatewayError::InvalidMerkleProof)?;

        // Check: Slot is already verified
        self.check_slot_is_done(&verifier_info.leaf)?;

        // Check: Merkle proof
        Self::verify_merkle_proof(verifier_info.leaf, &merkle_proof, verifier_set_merkle_root)?;

        // Check: Digital signature
        Self::verify_digital_signature(
            &verifier_info.leaf.signer_pubkey,
            payload_merkle_root,
            &verifier_info.signature,
        )?;

        // Update state
        self.accumulate_threshold(&verifier_info.leaf);
        self.mark_slot_done(&verifier_info.leaf)?;
        self.verify_or_initialize_verifier_set(verifier_set_merkle_root)?;

        Ok(())
    }

    /// Verifies or initializes the verifier set hash.
    /// Returns an error if the hash is already set and doesn't match.
    #[inline]
    fn verify_or_initialize_verifier_set(
        &mut self,
        expected_hash: &[u8; 32],
    ) -> Result<(), GatewayError> {
        if self.signing_verifier_set_hash == [0; 32] {
            self.signing_verifier_set_hash = *expected_hash;
            return Ok(());
        }

        if self.signing_verifier_set_hash != *expected_hash {
            return Err(GatewayError::InvalidDigitalSignature);
        }

        Ok(())
    }

    #[inline]
    fn check_slot_is_done(&self, signature_node: &VerifierSetLeaf) -> Result<(), GatewayError> {
        let signature_slots = self.signature_slots.view_bits::<Lsb0>();
        let position: usize = signature_node.position.into();
        let Some(slot) = signature_slots.get(position) else {
            // Index is out of bounds.
            return Err(GatewayError::SlotIsOutOfBounds);
        };
        // Check if signature slot was already verified.
        if *slot {
            return Err(GatewayError::SlotAlreadyVerified);
        }
        Ok(())
    }

    #[inline]
    fn verify_merkle_proof(
        signature_node: VerifierSetLeaf,
        merkle_proof: &rs_merkle::MerkleProof<SolanaSyscallHasher>,
        verifier_set_merkle_root: &[u8; 32],
    ) -> Result<(), GatewayError> {
        let leaf_hash = signature_node.hash::<SolanaSyscallHasher>();

        if merkle_proof.verify(
            *verifier_set_merkle_root,
            &[signature_node.position.into()],
            &[leaf_hash],
            signature_node.set_size.into(),
        ) {
            Ok(())
        } else {
            Err(GatewayError::InvalidMerkleProof)
        }
    }

    #[inline]
    #[allow(clippy::unimplemented)]
    fn verify_digital_signature(
        public_key: &PublicKey,
        message: &[u8; 32],
        signature: &Signature,
    ) -> Result<(), GatewayError> {
        let is_valid = match (signature, public_key) {
            (Signature::EcdsaRecoverable(signature), PublicKey::Secp256k1(pubkey)) => {
                verify_ecdsa_signature_with_prefix(pubkey, signature, message)
            }
            (Signature::Ed25519(_signature), PublicKey::Ed25519(_pubkey)) => {
                // TODO: Whenever we implement this, make sure to use the
                // `verify_eddsa_signature_with_prefix` function instead to account for the chain
                // prefix, similar to what we do for ECDSA above.
                unimplemented!()
            }
            _ => {
                solana_program::msg!(
                    "Error: Invalid combination of Secp256k1 and Ed25519 signature and public key"
                );
                false
            }
        };
        if is_valid {
            return Ok(());
        }

        Err(GatewayError::InvalidDigitalSignature)
    }

    #[inline]
    fn accumulate_threshold(&mut self, signature_node: &VerifierSetLeaf) {
        self.accumulated_threshold = self
            .accumulated_threshold
            .saturating_add(signature_node.signer_weight);

        // Check threshold
        if self.accumulated_threshold >= signature_node.quorum {
            self.accumulated_threshold = u128::MAX;
        }
    }

    #[inline]
    fn mark_slot_done(&mut self, signature_node: &VerifierSetLeaf) -> Result<(), GatewayError> {
        let signature_slots = self.signature_slots.view_bits_mut::<Lsb0>();
        let position: usize = signature_node.position.into();
        let Some(slot) = signature_slots.get_mut(position) else {
            // Index is out of bounds.
            return Err(GatewayError::SlotIsOutOfBounds);
        };
        // Check if signature slot was already verified.
        if *slot {
            return Err(GatewayError::SlotAlreadyVerified);
        }
        slot.commit(true);
        Ok(())
    }

    /// Iterator over the signature slots.
    pub fn slots_iter(&self) -> impl Iterator<Item = bool> + '_ {
        let signature_slots = self.signature_slots.view_bits::<Lsb0>();
        signature_slots.into_iter().map(|slot| *slot)
    }

    /// Bit slice into the signature array
    #[must_use]
    pub fn slots(&self) -> &BitSlice<u8> {
        self.signature_slots.view_bits::<Lsb0>()
    }
}

/// Verifies an ECDSA signature against a given message and public key using the
/// secp256k1 curve.
///
/// Returns `true` if the signature is valid and corresponds to the public key
/// and message; otherwise, returns `false`.
///
/// # Panics
///
/// This function will panic if the provided `pubkey` is not a valid compressed secp256k1 public key
/// (via `unwrap`).
#[must_use]
#[allow(clippy::unwrap_used)]
fn verify_ecdsa_signature(
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
    // Only values 27 and 28 are valid Ethereum recovery IDs
    let recovery_id = if *recovery_id == 27 || *recovery_id == 28 {
        recovery_id.saturating_sub(27)
    } else {
        solana_program::msg!("Invalid recovery ID: {} (must be 27 or 28)", recovery_id);
        return false;
    };

    // This is results in a Solana syscall.
    let secp256k1_recover =
        solana_program::secp256k1_recover::secp256k1_recover(message, recovery_id, signature);
    let Ok(recovered_uncompressed_pubkey) = secp256k1_recover else {
        solana_program::msg!("Failed to recover ECDSA signature");
        return false;
    };

    // Unwrap: provided pubkey is guaranteed to be secp256k1 key
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
#[must_use]
fn verify_eddsa_signature(
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

/// Prefix added to all signature verifications for Solana offchain messages
const SOLANA_OFFCHAIN_PREFIX: &[u8] = b"\xffsolana offchain";

/// Wrapper for `verify_ecdsa_signature` that adds the Solana offchain prefix.
///
/// This function prepends `\xffsolana offchain` to the message before verification.
/// Returns `true` if the signature is valid and corresponds to the public key
/// and prefixed message; otherwise, returns `false`.
///
/// # Panics
///
/// This function will panic if the provided `pubkey` is not a valid compressed secp256k1 public key
/// (via `unwrap`).
#[must_use]
pub fn verify_ecdsa_signature_with_prefix(
    pubkey: &axelar_solana_encoding::types::pubkey::Secp256k1Pubkey,
    signature: &axelar_solana_encoding::types::pubkey::EcdsaRecoverableSignature,
    message: &[u8; 32],
) -> bool {
    // Create prefixed message by concatenating prefix + original message
    let mut prefixed_message = Vec::with_capacity(SOLANA_OFFCHAIN_PREFIX.len() + message.len());
    prefixed_message.extend_from_slice(SOLANA_OFFCHAIN_PREFIX);
    prefixed_message.extend_from_slice(message);

    // Hash the prefixed message to get a 32-byte digest
    let hashed_message = SolanaSyscallHasher::hash(&prefixed_message);

    // Call the original verification function with the hashed prefixed message
    verify_ecdsa_signature(pubkey, signature, &hashed_message)
}

/// Wrapper for `verify_eddsa_signature` that adds the Solana offchain prefix.
///
/// This function prepends `\xffsolana offchain` to the message before verification.
/// Returns `true` if the signature is valid and corresponds to the public key
/// and prefixed message; otherwise, returns `false`.
#[deprecated(note = "Trying to verify Ed25519 signatures on-chain will exhaust the compute budget")]
#[must_use]
#[allow(deprecated)]
pub fn verify_eddsa_signature_with_prefix(
    pubkey: &axelar_solana_encoding::types::pubkey::Ed25519Pubkey,
    signature: &axelar_solana_encoding::types::pubkey::Ed25519Signature,
    message: &[u8; 32],
) -> bool {
    // Create prefixed message by concatenating prefix + original message
    let mut prefixed_message = Vec::with_capacity(SOLANA_OFFCHAIN_PREFIX.len() + message.len());
    prefixed_message.extend_from_slice(SOLANA_OFFCHAIN_PREFIX);
    prefixed_message.extend_from_slice(message);

    // Hash the prefixed message to get a 32-byte digest
    let hashed_message = SolanaSyscallHasher::hash(&prefixed_message);

    // Call the original verification function with the hashed prefixed message
    verify_eddsa_signature(pubkey, signature, &hashed_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_solana_encoding::hasher::{Hasher, NativeHasher};
    use axelar_solana_encoding::types::execute_data::SigningVerifierSetInfo;
    use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
    use axelar_solana_encoding::types::verifier_set::VerifierSetLeaf;
    use axelar_solana_encoding::{rs_merkle, LeafHash};
    use rand::Rng;

    #[test]
    fn test_initialize_when_hash_is_zero() {
        let mut verification = SignatureVerification::default();
        let new_hash = [42_u8; 32];

        let result = verification.verify_or_initialize_verifier_set(&new_hash);

        assert!(result.is_ok());
        assert_eq!(verification.signing_verifier_set_hash, new_hash);
    }

    #[test]
    fn test_verify_success_when_hashes_match() {
        let expected_hash = [42_u8; 32];
        let mut verification = SignatureVerification {
            signing_verifier_set_hash: expected_hash,
            ..Default::default()
        };

        let result = verification.verify_or_initialize_verifier_set(&expected_hash);

        assert!(result.is_ok());
        assert_eq!(verification.signing_verifier_set_hash, expected_hash);
    }

    #[test]
    fn test_verify_fails_when_hashes_mismatch() {
        let initial_hash = [42_u8; 32];
        let different_hash = [24_u8; 32];
        let mut verification = SignatureVerification {
            signing_verifier_set_hash: initial_hash,
            ..Default::default()
        };

        let result = verification.verify_or_initialize_verifier_set(&different_hash);

        assert_eq!(result, Err(GatewayError::InvalidDigitalSignature));
        // Hash should remain unchanged after failure
        assert_eq!(verification.signing_verifier_set_hash, initial_hash);
    }

    #[test]
    fn test_check_slot_is_done_returns_slot_already_verified_error() {
        let mut verification = SignatureVerification::default();

        // Set the bit at position 0 to indicate it's already verified
        verification.signature_slots[0] = 0b0000_0001;

        let verifier_leaf = VerifierSetLeaf {
            signer_pubkey: PublicKey::Secp256k1([0; 33]),
            signer_weight: 1,
            position: 0u8.into(),
            quorum: 1,
            set_size: 1u8.into(),
            domain_separator: [0; 32],
            nonce: 0,
        };

        assert_eq!(
            verification.check_slot_is_done(&verifier_leaf),
            Err(GatewayError::SlotAlreadyVerified)
        );
    }

    #[test]
    fn test_process_signature_returns_slot_already_verified_error() {
        // Create ECDSA keypair
        let (secret_key, public_key_bytes) = {
            let mut rng = rand::thread_rng();
            let secret_key_bytes: [u8; 32] = rng.gen();
            let secret_key =
                libsecp256k1::SecretKey::parse(&secret_key_bytes).expect("valid secret key");
            let public_key = libsecp256k1::PublicKey::from_secret_key(&secret_key);
            let public_key_bytes = public_key.serialize_compressed();
            (secret_key, public_key_bytes)
        };

        // Create verifier set leaf
        let verifier_leaf = {
            let mut rng = rand::thread_rng();
            VerifierSetLeaf {
                signer_pubkey: PublicKey::Secp256k1(public_key_bytes),
                position: 0u8.into(),
                signer_weight: rng.gen(),
                quorum: rng.gen(),
                set_size: 1u8.into(),
                domain_separator: rng.gen(),
                nonce: rng.gen(),
            }
        };

        // Create Merkle tree and proof
        let (merkle_root, proof_bytes) = {
            let leaf_hash = verifier_leaf.hash::<NativeHasher>();
            let tree = rs_merkle::MerkleTree::<NativeHasher>::from_leaves(&[leaf_hash]);
            let merkle_root = tree.root().expect("tree should have root");
            let merkle_proof = tree.proof(&[0]);
            let proof_bytes = merkle_proof.to_bytes();
            (merkle_root, proof_bytes)
        };

        // Create payload and signature
        let (payload_merkle_root, signature_array) = {
            let mut rng = rand::thread_rng();
            let payload_merkle_root: [u8; 32] = rng.gen();

            // Create the prefixed message that will actually be signed
            let mut prefixed_message =
                Vec::with_capacity(SOLANA_OFFCHAIN_PREFIX.len() + payload_merkle_root.len());
            prefixed_message.extend_from_slice(SOLANA_OFFCHAIN_PREFIX);
            prefixed_message.extend_from_slice(&payload_merkle_root);
            let hashed_prefixed_message = NativeHasher::hash(&prefixed_message);

            let message = libsecp256k1::Message::parse(&hashed_prefixed_message);
            let (signature, recovery_id) = libsecp256k1::sign(&message, &secret_key);
            let mut signature_bytes = signature.serialize().to_vec();
            // Convert recovery_id from libsecp256k1 format (0-3) to Ethereum format (27-28)
            signature_bytes.push(recovery_id.serialize() + 27);
            let signature_array: [u8; 65] = signature_bytes.try_into().unwrap();
            (payload_merkle_root, signature_array)
        };

        // Assemble verifier info
        let verifier_info = SigningVerifierSetInfo {
            leaf: verifier_leaf,
            signature: Signature::EcdsaRecoverable(signature_array),
            merkle_proof: proof_bytes,
        };

        let mut verification = SignatureVerification::default();

        // First call should succeed and mark the slot as verified
        assert!(verification
            .process_signature(&verifier_info, &merkle_root, &payload_merkle_root)
            .is_ok());

        // Second call with the same input should fail with SlotAlreadyVerified
        assert_eq!(
            verification.process_signature(&verifier_info, &merkle_root, &payload_merkle_root),
            Err(GatewayError::SlotAlreadyVerified)
        );
    }
}
