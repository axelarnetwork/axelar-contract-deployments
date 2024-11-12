//! Multi-step signature verification.

use std::mem;

use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, SolanaSyscallHasher};
use axelar_rkyv_encoding::types::{PublicKey, Signature, VerifierSetLeafNode};
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
        signature_node: VerifierSetLeafNode<SolanaSyscallHasher>,
        merkle_proof: &MerkleProof<SolanaSyscallHasher>,
        verifier_set_merkle_root: &[u8; 32],
        payload_merkle_root: &[u8; 32],
        signature: &Signature,
        signature_verifier: &impl SignatureVerifier,
    ) -> Result<(), SignatureVerificationError> {
        // Check: Slot is already verified
        self.check_slot_is_done(signature_node)?;

        // Check: Merkle proof
        Self::verify_merkle_proof(signature_node, merkle_proof, verifier_set_merkle_root)?;

        // Check: Digital signature
        Self::verify_digital_signature(
            &signature_node.signer_pubkey,
            payload_merkle_root,
            signature,
            signature_verifier,
        )?;

        // Update state
        self.accumulate_threshold(signature_node);
        self.mark_slot_done(signature_node)?;
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
        signature_node: VerifierSetLeafNode<SolanaSyscallHasher>,
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
        signature_node: VerifierSetLeafNode<SolanaSyscallHasher>,
        merkle_proof: &MerkleProof<SolanaSyscallHasher>,
        verifier_set_merkle_root: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        let leaf_hash: [u8; 32] = signature_node.into();

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
    fn accumulate_threshold(&mut self, signature_node: VerifierSetLeafNode<SolanaSyscallHasher>) {
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
        signature_node: VerifierSetLeafNode<SolanaSyscallHasher>,
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

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use axelar_rkyv_encoding::hasher::merkle_trait::Merkle;
    use axelar_rkyv_encoding::test_fixtures::{
        random_bytes, random_signature, random_valid_verifier_set_fixed_size,
    };
    use axelar_rkyv_encoding::types::VerifierSet;
    use rand::rngs::OsRng;
    use rand::Rng;

    use super::*;

    struct MockSignatureVerifier(bool);

    impl SignatureVerifier for MockSignatureVerifier {
        fn verify_signature(
            &self,
            _signature: &Signature,
            _public_key: &PublicKey,
            _message: &[u8; 32],
        ) -> bool {
            self.0
        }
    }

    struct MerkleIter {
        root: [u8; 32],
        leaves: VecDeque<VerifierSetLeafNode<SolanaSyscallHasher>>,
        proofs: VecDeque<MerkleProof<SolanaSyscallHasher>>,
    }

    impl MerkleIter {
        fn new(verifier_set: &VerifierSet) -> Self {
            let root = Merkle::<SolanaSyscallHasher>::calculate_merkle_root(verifier_set).unwrap();
            let leaves = verifier_set.merkle_leaves().collect();
            let proofs = verifier_set.merkle_proofs().collect();
            Self {
                root,
                leaves,
                proofs,
            }
        }
    }

    impl Iterator for MerkleIter {
        type Item = MerkleItems;

        fn next(&mut self) -> Option<Self::Item> {
            let leaf = self.leaves.pop_front()?;
            let proof = self.proofs.pop_front()?;
            Some(MerkleItems {
                root: self.root,
                leaf,
                proof,
            })
        }
    }

    struct MerkleItems {
        root: [u8; 32],
        leaf: VerifierSetLeafNode<SolanaSyscallHasher>,
        proof: MerkleProof<SolanaSyscallHasher>,
    }

    fn random_merkle_items() -> impl Iterator<Item = MerkleItems> {
        let num_signers = OsRng.gen_range(40..120);
        let verifier_set = random_valid_verifier_set_fixed_size(num_signers);
        MerkleIter::new(&verifier_set)
    }

    #[test]
    fn test_process_signature() {
        // Setup
        let mut session = SignatureVerification::default();
        let MerkleItems { root, leaf, proof } = random_merkle_items().next().unwrap();
        let mock = MockSignatureVerifier(true);

        // confidence check
        assert!(
            !session.slot(leaf.position as usize).expect("existing slot"),
            "uninitialized slot should be unset"
        );

        // verify
        session
            .process_signature(
                leaf,
                &proof,
                &root,
                &random_bytes(),
                &random_signature(),
                &mock,
            )
            .unwrap();

        // check outcome
        assert_eq!(session.accumulated_threshold, leaf.signer_weight);
        assert!(session.slot(leaf.position as usize).unwrap())
    }

    #[test]
    fn test_sufficient_threshold() {
        // Setup
        let mut session = SignatureVerification::default();
        let mock = MockSignatureVerifier(true);
        let mut real_accumulated_weight = 0u128;
        let mut quorum: Option<u128> = None;

        // verify
        for MerkleItems { root, leaf, proof } in random_merkle_items() {
            session
                .process_signature(
                    leaf,
                    &proof,
                    &root,
                    &random_bytes(),
                    &random_signature(),
                    &mock,
                )
                .unwrap();
            real_accumulated_weight += leaf.signer_weight;
            quorum.get_or_insert(leaf.quorum);
        }

        // by now, this session should have obtained the required threshold.
        assert_eq!(real_accumulated_weight, quorum.unwrap()); // confidence check
        assert!(session.is_valid());
        assert_eq!(session.accumulated_threshold, u128::MAX);
    }

    #[test]
    fn test_repeated_signature() {
        // setup
        let mut session = SignatureVerification::default();
        let MerkleItems { root, leaf, proof } = random_merkle_items().next().unwrap();
        let mock = MockSignatureVerifier(true);

        // run
        session
            .process_signature(
                leaf,
                &proof,
                &root,
                &random_bytes(),
                &random_signature(),
                &mock,
            )
            .unwrap();

        // run twice
        let err = session
            .process_signature(
                leaf,
                &proof,
                &root,
                &random_bytes(),
                &random_signature(),
                &mock,
            )
            .expect_err("a repeated signature should result in a verification error");
        assert!(matches!(
            err,
            SignatureVerificationError::SlotAlreadyVerified(0)
        ));

        // state should reflect just a single validation
        assert_eq!(session.accumulated_threshold, leaf.signer_weight);
        assert_eq!(session.slot(0), Some(true));
        assert!(!session.is_valid())
    }

    #[test]
    fn test_invalid_merkle_proof() {
        // setup
        let mut session = SignatureVerification::default();
        let MerkleItems { leaf, proof, .. } = random_merkle_items().next().unwrap();
        let invalid_root = random_bytes();
        let mock = MockSignatureVerifier(true);

        // run
        let err = session
            .process_signature(
                leaf,
                &proof,
                &invalid_root,
                &random_bytes(),
                &random_signature(),
                &mock,
            )
            .expect_err("an invalid root should result in a verification error");
        assert!(matches!(
            err,
            SignatureVerificationError::InvalidMerkleProof
        ));

        // assert no state changes
        assert_eq!(session.accumulated_threshold, 0);
        assert_eq!(session.signature_slots, [0u8; 32]);
        assert!(!session.is_valid())
    }

    #[test]
    fn test_invalid_digital_signature() {
        // setup
        let mut session = SignatureVerification::default();
        let MerkleItems { root, leaf, proof } = random_merkle_items().next().unwrap();
        let mock = MockSignatureVerifier(false);

        // run
        let err = session
            .process_signature(
                leaf,
                &proof,
                &root,
                &random_bytes(),
                &random_signature(),
                &mock,
            )
            .expect_err("an invalid signature should result in a verification error");
        assert!(matches!(
            err,
            SignatureVerificationError::InvalidDigitalSignature
        ));

        // assert no state changes
        assert_eq!(session.accumulated_threshold, 0);
        assert_eq!(session.signature_slots, [0u8; 32]);
        assert!(!session.is_valid())
    }
}
