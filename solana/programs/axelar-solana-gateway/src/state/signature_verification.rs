//! Multi-step signature verification.

use std::mem;

use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::SigningVerifierSetInfo;
use axelar_solana_encoding::types::pubkey::{PublicKey, Signature};
use axelar_solana_encoding::types::verifier_set::VerifierSetLeaf;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use bitvec::view::BitView;
use bytemuck::{Pod, Zeroable};

use super::verifier_set_tracker::VerifierSetHash;

/// Controls the signature verification session for a given payload.
#[repr(C)]
#[derive(Default, Clone, Copy, Pod, Zeroable, Debug, Eq, PartialEq)]
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
    /// verifier set was the one that actually .
    pub signing_verifier_set_hash: VerifierSetHash,
}

/// Errors that can happen during a signature verification session.
#[derive(Debug, thiserror::Error)]
pub enum SignatureVerificationError {
    /// Used when a signature index is too high.
    #[error("Slot #{0} is out of bounds")]
    SlotIsOutOfBounds(usize),

    /// Used when someone tries to verify a signature that has already been
    /// verified before.
    #[error("Slot #{0} has been previously verified")]
    SlotAlreadyVerified(usize),

    /// Used when the Merkle inclusion proof fails to verify against the given
    /// root.
    #[error("Signer is not a member of the active verifier set")]
    InvalidMerkleProof,

    /// Used when the internal digital signature verification fails.
    #[error("Digital signature verification failed")]
    InvalidDigitalSignature,
}

impl SignatureVerification {
    /// The length, in bytes, of the serialized representation for this type.
    pub const LEN: usize = mem::size_of::<Self>();

    /// Returns `true` if a sufficient number of signatures have been verified.
    pub fn is_valid(&self) -> bool {
        self.accumulated_threshold == u128::MAX
    }

    /// Fully process a submitted signature.
    pub fn process_signature(
        &mut self,
        verifier_info: SigningVerifierSetInfo,
        verifier_set_merkle_root: &[u8; 32],
        payload_merkle_root: &[u8; 32],
        signature_verifier: &impl SignatureVerifier,
    ) -> Result<(), SignatureVerificationError> {
        let merkle_proof =
            rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&verifier_info.merkle_proof)
                .unwrap();
        // Check: Slot is already verified
        self.check_slot_is_done(&verifier_info.leaf)?;

        // Check: Merkle proof
        Self::verify_merkle_proof(verifier_info.leaf, &merkle_proof, verifier_set_merkle_root)?;

        // Check: Digital signature
        Self::verify_digital_signature(
            &verifier_info.leaf.signer_pubkey,
            payload_merkle_root,
            &verifier_info.signature,
            signature_verifier,
        )?;

        // Update state
        self.accumulate_threshold(&verifier_info.leaf);
        self.mark_slot_done(&verifier_info.leaf)?;
        if self.signing_verifier_set_hash == [0; 32] {
            self.signing_verifier_set_hash = *verifier_set_merkle_root;
        } else if &self.signing_verifier_set_hash != verifier_set_merkle_root {
            return Err(SignatureVerificationError::InvalidDigitalSignature);
        }

        Ok(())
    }

    #[inline]
    fn check_slot_is_done(
        &self,
        signature_node: &VerifierSetLeaf,
    ) -> Result<(), SignatureVerificationError> {
        let signature_slots = self.signature_slots.view_bits::<Lsb0>();
        let position = signature_node.position as usize;
        let Some(slot) = signature_slots.get(position) else {
            // Index is out of bounds.
            return Err(SignatureVerificationError::SlotIsOutOfBounds(position));
        };
        // Check if signature slot was already verified.
        if *slot {
            return Err(SignatureVerificationError::SlotAlreadyVerified(position));
        }
        Ok(())
    }

    #[inline]
    fn verify_merkle_proof(
        signature_node: VerifierSetLeaf,
        merkle_proof: &rs_merkle::MerkleProof<SolanaSyscallHasher>,
        verifier_set_merkle_root: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        let leaf_hash = signature_node.hash::<SolanaSyscallHasher>();

        if merkle_proof.verify(
            *verifier_set_merkle_root,
            &[signature_node.position as usize],
            &[leaf_hash],
            signature_node.set_size as usize,
        ) {
            Ok(())
        } else {
            Err(SignatureVerificationError::InvalidMerkleProof)
        }
    }

    #[inline]
    fn verify_digital_signature(
        public_key: &PublicKey,
        message: &[u8; 32],
        signature: &Signature,
        signature_verifier: &impl SignatureVerifier,
    ) -> Result<(), SignatureVerificationError> {
        if signature_verifier.verify_signature(signature, public_key, message) {
            Ok(())
        } else {
            Err(SignatureVerificationError::InvalidDigitalSignature)
        }
    }

    #[inline]
    fn accumulate_threshold(&mut self, signature_node: &VerifierSetLeaf) {
        self.accumulated_threshold = self
            .accumulated_threshold
            .saturating_add(signature_node.signer_weight);

        // Check threshold
        if self.accumulated_threshold >= signature_node.quorum {
            self.accumulated_threshold = u128::MAX
        }
    }
    #[inline]
    fn mark_slot_done(
        &mut self,
        signature_node: &VerifierSetLeaf,
    ) -> Result<(), SignatureVerificationError> {
        let signature_slots = self.signature_slots.view_bits_mut::<Lsb0>();
        let position = signature_node.position as usize;
        let Some(slot) = signature_slots.get_mut(position) else {
            // Index is out of bounds.
            return Err(SignatureVerificationError::SlotIsOutOfBounds(position));
        };
        // Check if signature slot was already verified.
        if *slot {
            return Err(SignatureVerificationError::SlotAlreadyVerified(position));
        }
        slot.commit(true);
        Ok(())
    }

    /// Returns the slot for a given position.
    #[inline]
    pub fn slot(&self, position: usize) -> Option<bool> {
        self.signature_slots
            .view_bits::<Lsb0>()
            .get(position)
            .as_deref()
            .copied()
    }

    /// Iterator over the signature slots.
    pub fn slots_iter(&self) -> impl Iterator<Item = bool> + '_ {
        let signature_slots = self.signature_slots.view_bits::<Lsb0>();
        signature_slots.into_iter().map(|slot| *slot)
    }

    /// Bit slice into the signature array
    pub fn slots(&self) -> &BitSlice<u8> {
        self.signature_slots.view_bits::<Lsb0>()
    }
}

/// A trait for types that can verify digital signatures.
pub trait SignatureVerifier {
    /// Verifies if the `signature` was created using the `public_key` for the
    /// given `message`.
    fn verify_signature(
        &self,
        signature: &Signature,
        public_key: &PublicKey,
        message: &[u8; 32],
    ) -> bool;
}
