//! Multi-step, batch-oriented, Merkle tree-based signature verification.

use std::mem::size_of;

use arrayref::{array_refs, mut_array_refs};
use axelar_rkyv_encoding::types::ArchivedProof;
use bitvec::order::Lsb0;
use bitvec::{bitarr, BitArr};
use merkle_tree::{MerkleProof, MerkleTree};
use solana_program::hash::hashv;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

const MAX_SIGNATURES: usize = u8::MAX as usize;

/// TODO: turn this into a new type, so users don't pass invalid data by
/// accident.
type Hash = [u8; 32];

/// Represents all data required for a leaf node to be inserted in the
/// [`SignatureVerification`]'s Merkle tree.
pub struct SignatureNode<'ctx, S, K>
where
    S: AsRef<[u8]>,
    K: AsRef<[u8]>,
{
    /// Signer's signature.
    signature_bytes: S,
    /// Signer's public key.
    public_key_bytes: K,
    /// Signer's weight.
    signer_weight: u128,
    /// Signer's position within the signer set used for this message batch.
    signer_index: u8,
    /// Details about the message batch this signature belongs to.
    batch_context: &'ctx BatchContext,
}

impl<'ctx, S, K> SignatureNode<'ctx, S, K>
where
    S: AsRef<[u8]>,
    K: AsRef<[u8]>,
{
    /// Creates a new [`SignatureNode`].
    ///
    /// Will panic if `signer_index` is greater than
    /// `batch_context.signer_count`.
    pub fn new(
        signature_bytes: S,
        public_key_bytes: K,
        signer_weight: u128,
        signer_index: u8,
        batch_context: &'ctx BatchContext,
    ) -> Self {
        assert!(
            signer_index <= batch_context.signer_count,
            "signer index cannot be greater than the total signer count"
        );
        Self {
            signature_bytes,
            public_key_bytes,
            signer_weight,
            signer_index,
            batch_context,
        }
    }

    /// Destructure this `[SignatureNode]` value into its constituent parts.
    pub fn into_parts(self) -> (S, K, u128, u8) {
        let SignatureNode {
            signature_bytes,
            public_key_bytes,
            signer_weight,
            signer_index,
            batch_context: _,
        } = self;
        (
            signature_bytes,
            public_key_bytes,
            signer_weight,
            signer_index,
        )
    }

    /// Uses Solana's [`hashv`] syscall to hash this [`SignatureNode`].
    ///
    /// Every field and subfields of the [`SignatureNode`] should be included in
    /// the hash.
    pub fn hash(&self) -> Hash {
        hashv(&[
            // Leaf Node prefix
            b"00",
            // SignatureNode fields
            self.signature_bytes.as_ref(),
            self.public_key_bytes.as_ref(),
            &self.signer_weight.to_le_bytes(),
            &[self.signer_index],
            // BatchContext fields
            self.batch_context.gateway_root_pda.as_ref(),
            &self.batch_context.domain_separator,
            &[self.batch_context.signer_count],
            &self.batch_context.message_hash,
        ])
        .to_bytes()
    }
}

/// General information about the message batch.
#[derive(Eq, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct BatchContext {
    /// The Gateway Root PDA.
    pub gateway_root_pda: Pubkey,
    /// Domain separator.
    ///
    /// Stored in the root PDA. This will disallow hash collisions from devnet /
    /// mainnet if the same signer appears on both chains.
    pub domain_separator: [u8; 32],
    /// Required signer weight to validate the current batch.
    pub threshold: u128,
    /// The message hash itself (the one that was signed).
    pub message_hash: Hash,
    /// Total number of signers in the command batch.
    pub signer_count: u8,
    _pad: [u8; 15],
}

impl BatchContext {
    /// The length, in bytes, of the serialized representation for this type.
    pub const LEN: usize = size_of::<Self>();

    /// Creates a new `BatchContext` instance.
    pub fn new(
        gateway_root_pda: Pubkey,
        domain_separator: [u8; 32],
        threshold: u128,
        message_hash: Hash,
        signer_count: u8,
    ) -> Self {
        Self {
            gateway_root_pda,
            domain_separator,
            threshold,
            message_hash,
            signer_count,
            _pad: [0; 15],
        }
    }

    /// Serializes the `BatchContext` instance into a fixed-size byte array.
    pub fn serialize_into(&self, bytes: &mut [u8; Self::LEN]) {
        let (gateway_root_pda, domain_separator, threshold, message_hash, signer_count, _pad) =
            mut_array_refs![bytes, 32, 32, 16, 32, 1, 15];
        gateway_root_pda.copy_from_slice(self.gateway_root_pda.as_ref());
        domain_separator.copy_from_slice(&self.domain_separator);
        threshold.copy_from_slice(&self.threshold.to_le_bytes());
        message_hash.copy_from_slice(&self.message_hash);
        signer_count[0] = self.signer_count;
    }

    /// Deserializes a byte array into a `BatchContext` instance.
    pub fn deserialize(bytes: &[u8; Self::LEN]) -> Self {
        let (gateway_root_pda, domain_separator, threshold, message_hash, signer_count, _pad) =
            array_refs![bytes, 32, 32, 16, 32, 1, 15];
        Self {
            gateway_root_pda: Pubkey::from(*gateway_root_pda),
            domain_separator: *domain_separator,
            threshold: u128::from_le_bytes(*threshold),
            message_hash: *message_hash,
            signer_count: signer_count[0],
            _pad: [0; 15],
        }
    }
}

/// Controls the `Proof` signature verification phase.
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub struct SignatureVerification {
    /// The Merkle root representing all signatures in a `Proof`.
    merkle_root: Hash,

    /// Number of signature (leaf) nodes in the original Merkle Tree.
    ///
    /// Used as part of the inclusion proof verification.
    total_leaves_count: u8,

    /// Remaining signer threshold required to validate the `Proof`.
    ///
    /// Is decremented on each successful verification.
    /// A value of zero equals the `Proof` being valid.
    remaining_threshold: u128,

    /// Original required threshold to validate the `Proof`.
    ///
    /// Used only in tests.
    #[cfg(test)]
    original_threshold: u128,

    /// A bit field used to track which signatures have been verified.
    ///
    /// Initially, all bits are set to zero. When a signature is verified, its
    /// corresponding bit is flipped to one. This prevents the same signature
    /// from being verified more than once, avoiding deliberate attempts to
    /// decrement the remaining threshold.
    signature_slots: BitArr!(for MAX_SIGNATURES, in u8, Lsb0),
}

impl SignatureVerification {
    /// The length, in bytes, of the serialized representation for this type.
    pub const LEN: usize = 84;

    /// Creates a new [`SignatureVerification`] with all `signature_slots` set
    /// to zero.
    pub fn new(merkle_root: Hash, total_leaves_count: u8, threshold: u128) -> Self {
        Self {
            merkle_root,
            total_leaves_count,
            remaining_threshold: threshold,
            #[cfg(test)]
            original_threshold: threshold,
            signature_slots: bitarr![u8, Lsb0; 0; MAX_SIGNATURES],
        }
    }

    /// Returns the Merkle Tree for the given signature (leaf) nodes.
    ///
    /// Intended to be used by off-chain agents to prepare their instructions.
    pub fn build_merkle_tree<S, K>(leaves: &[SignatureNode<'_, S, K>]) -> MerkleTree
    where
        S: AsRef<[u8]>,
        K: AsRef<[u8]>,
    {
        let mut hashes = Vec::with_capacity(leaves.len());
        hashes.extend(leaves.iter().map(|leaf| leaf.hash()));
        MerkleTree::from_leaves(&hashes)
    }

    /// Returns the stored merkle root.
    pub fn root(&self) -> [u8; 32] {
        self.merkle_root
    }

    /// Returns the remaining threshold for [`SignatureVerification`] be
    /// considered valid.
    pub fn remaining_threshold(&self) -> u128 {
        self.remaining_threshold
    }

    /// Returns `true` if a sufficient number of signatures have been verified.
    pub fn is_valid(&self) -> bool {
        self.remaining_threshold == 0
    }

    /// Checks the proof of inclusion for a signature at a specified position,
    /// as well as the validity of the given signature for the provided
    /// public key and message.
    ///
    /// The `signature_bytes` will be hashed internally using the
    /// [`SignatureVerification::hash`] method.
    ///
    /// The actual verification of the signature is delegated to
    /// `signature_verifier`.
    pub fn verify_signature<S, K>(
        &mut self,
        signature_node: &SignatureNode<'_, S, K>,
        proof: MerkleProof,
        signature_verifier: impl SignatureVerifier<S, K>,
    ) -> bool
    where
        S: AsRef<[u8]>,
        K: AsRef<[u8]>,
    {
        let Some(slot) = self
            .signature_slots
            .get_mut(signature_node.signer_index as usize)
        else {
            // Index is out of bounds.
            return false;
        };

        // Check if signature slot was already verified.
        if *slot {
            return false;
        }

        // Obtain the signature node hash.
        let leaf_hash = signature_node.hash();

        // Check signature proof of inclusion.
        if !proof.verify(
            self.merkle_root,
            &[signature_node.signer_index as usize],
            &[leaf_hash],
            self.total_leaves_count as usize,
        ) {
            return false;
        }

        // Verify signature
        let Some(signer_threshold) = signature_verifier.verify_signature(
            &signature_node.signature_bytes,
            &signature_node.public_key_bytes,
            &signature_node.batch_context.message_hash,
        ) else {
            return false;
        };

        // Decrement threshold
        self.remaining_threshold = self.remaining_threshold.saturating_sub(signer_threshold);

        // Update the signature slot
        slot.commit(true);
        true
    }

    #[cfg(test)]
    /// Serialize this [`SignatureValidation`] instance into an array of bytes.
    pub fn serialize(&self) -> [u8; Self::LEN] {
        let mut bytes = [0u8; Self::LEN];
        self.serialize_into(&mut bytes);
        bytes
    }

    /// Serialize this [`SignatureValidation`] instance into a mutable slice of
    /// bytes.
    pub fn serialize_into(&self, bytes: &mut [u8; Self::LEN]) {
        let (signature_slots, merkle_root, remaining_threshold, leaves_count, _pad) =
            mut_array_refs![bytes, 32, 32, 16, 1, 3];

        // Copy signature_slots (32 bytes)
        signature_slots.copy_from_slice(&self.signature_slots.data);

        // Copy merkle_root (32 bytes)
        merkle_root.copy_from_slice(&self.merkle_root);

        // Copy remaining_threshold (16 bytes)
        remaining_threshold.copy_from_slice(&self.remaining_threshold.to_le_bytes());

        // Copy total_leaves_count (1 byte)
        leaves_count[0] = self.total_leaves_count;
    }

    /// Deserialize a [`SignatureValidation`] value from an array of bytes
    pub fn deserialize(bytes: &[u8; Self::LEN]) -> Self {
        let (signature_slots, merkle_root, remaining_threshold, leaves_count, _pad) =
            array_refs![bytes, 32, 32, 16, 1, 3];

        Self {
            merkle_root: *merkle_root,
            total_leaves_count: leaves_count[0],
            remaining_threshold: u128::from_le_bytes(*remaining_threshold),
            #[cfg(test)]
            original_threshold: 0,
            signature_slots: (*signature_slots).into(),
        }
    }
}

/// A trait for types that can verify digital signatures.
pub trait SignatureVerifier<S, K> {
    /// Verifies if the `signature` was created using the `public_key` for the
    /// given `message`.
    ///
    /// Returns an `Option<u128>` with that signer's weight if the verification
    /// is successful, or `None` otherwise.
    fn verify_signature(&self, signature: &S, public_key: &K, message: &Hash) -> Option<u128>;
}

/// Type definitions for the Merkle Tree primitives used by
/// [`SignatureVerification`]
pub mod merkle_tree {
    use super::*;

    /// Merkle Tree implementation that uses Solana's `hashv` syscall to merge
    /// its nodes.
    pub type MerkleTree = rs_merkle::MerkleTree<SolanaSyscallHasher>;

    /// Merkle Proof implementation that uses Solana's `hashv` syscall to merge
    /// its nodes.
    pub type MerkleProof = rs_merkle::MerkleProof<SolanaSyscallHasher>;

    /// Hashing algorithm that defers to Solana's `hashv` syscall.
    #[derive(Clone)]
    pub struct SolanaSyscallHasher;

    impl rs_merkle::Hasher for SolanaSyscallHasher {
        type Hash = [u8; 32];

        fn hash(data: &[u8]) -> Self::Hash {
            hashv(&[data]).to_bytes()
        }

        /// This implementation deviates from the default for several reasons:
        /// 1. It prefixes intermediate nodes before hashing to prevent second
        ///    preimage attacks. This distinguishes leaf nodes from
        ///    intermediates, blocking attempts to craft alternative trees with
        ///    the same root hash using malicious hashes.
        /// 2. If the left node doesn't have a sibling it is concatenated to
        ///    itself and then hashed instead of just being propagated to the
        ///    next level.
        /// 3. It uses arrays instead of vectors to avoid heap allocations.
        fn concat_and_hash(left: &Self::Hash, right: Option<&Self::Hash>) -> Self::Hash {
            let mut concatenated: [u8; 65] = [0; 65];
            let (prefix, left_node, right_node) = mut_array_refs![&mut concatenated, 1, 32, 32];
            prefix[0] = 1;
            left_node.copy_from_slice(left);
            right_node.copy_from_slice(right.unwrap_or(left));
            Self::hash(&concatenated)
        }
    }
}

/// Parses a `Proof` and returns a `BatchContext` instance.
pub fn batch_context_from_proof(
    gateway_root_pda: Pubkey,
    domain_separator: [u8; 32],
    proof: &ArchivedProof,
    payload_hash: [u8; 32],
) -> Result<BatchContext, ProgramError> {
    let signer_count: u8 = proof
        .signers_with_signatures
        .len()
        .try_into()
        .map_err(|_| {
            msg!("Proof has more than 256 signers");
            ProgramError::InvalidAccountData
        })?;

    let batch_context = BatchContext::new(
        gateway_root_pda,
        domain_separator,
        (&proof.threshold).into(),
        payload_hash,
        signer_count,
    );
    Ok(batch_context)
}

#[cfg(test)]
mod tests {

    use axelar_rkyv_encoding::test_fixtures::random_bytes;
    use lazy_static::lazy_static;

    use super::*;

    type Stub = [u8; 32];
    type SignatureStub = [u8; 65];

    type TestSignatureNode = SignatureNode<'static, SignatureStub, Stub>;

    /// Mock signature verifier that always returns `Some(1)`.
    struct SignatureAlwaysValid;

    impl SignatureVerifier<SignatureStub, Stub> for SignatureAlwaysValid {
        fn verify_signature(
            &self,
            _signature: &SignatureStub,
            _public_key: &Stub,
            _message: &Hash,
        ) -> Option<u128> {
            Some(1)
        }
    }

    /// Mock signature verifier that always returns `None`.
    struct SignatureAlwaysInvalid;
    impl SignatureVerifier<SignatureStub, Stub> for SignatureAlwaysInvalid {
        fn verify_signature(
            &self,
            _signature: &SignatureStub,
            _public_key: &Stub,
            _message: &Hash,
        ) -> Option<u128> {
            None
        }
    }

    struct TestCase<const NUM_SIGNATURES: usize> {
        sig_verification: SignatureVerification,
        signatures: [TestSignatureNode; NUM_SIGNATURES],
        merkle_tree: MerkleTree,
    }

    lazy_static! {
        static ref TEST_BATCH_CONTEXT: BatchContext = BatchContext {
            gateway_root_pda: Pubkey::new_unique(),
            domain_separator: random_bytes(),
            signer_count: MAX_SIGNATURES as u8,
            message_hash: random_bytes(),
            threshold: MAX_SIGNATURES as u128,
            _pad: [0; 15]
        };
    }

    /// Creates random test data.
    fn setup<const NUM_SIGNATURES: usize>() -> TestCase<NUM_SIGNATURES> {
        let signature_nodes = std::array::from_fn(|pos| TestSignatureNode {
            signature_bytes: random_bytes(),
            public_key_bytes: random_bytes(),
            signer_weight: 1,
            signer_index: pos as u8,
            batch_context: &TEST_BATCH_CONTEXT,
        });

        let threshold = NUM_SIGNATURES as u128; // each signer weight = 1
        let merkle_tree = SignatureVerification::build_merkle_tree(&signature_nodes);
        let merkle_root = merkle_tree
            .root()
            .expect("the merkle tree should have its root");
        let sig_verification =
            SignatureVerification::new(merkle_root, NUM_SIGNATURES as u8, threshold);

        // Post-init checks, basic premises
        assert_eq!(
            sig_verification.total_leaves_count as usize,
            signature_nodes.len(),
            "total leaves count should be equal to what was passed when calling new()"
        );
        assert!(
            !sig_verification.is_valid(),
            "signature verification should not be valid right after initialization"
        );
        assert!(
            sig_verification.signature_slots.iter().all(|x| !x),
            "signature slots should be unset right after initialization"
        );

        TestCase {
            sig_verification,
            signatures: signature_nodes,
            merkle_tree,
        }
    }

    /// Check that internal state hasn't change after failed signature
    /// verification attempts.
    fn assert_unchanged(sig_verification: SignatureVerification) {
        assert!(
            !sig_verification.is_valid(),
            "signature verification should still not be valid after failed verification attempts"
        );
        assert!(
            sig_verification.signature_slots.iter().all(|x| !x),
            "signature slots should all unset after failed verification attempts"
        );

        assert_eq!(
            sig_verification.remaining_threshold, sig_verification.original_threshold,
            "remaining threshold should not change after failed verification attempts"
        )
    }

    #[test]
    fn test_basic_operation() {
        let TestCase {
            mut sig_verification,
            signatures,
            merkle_tree,
        } = setup::<MAX_SIGNATURES>();
        let original_threshold = sig_verification.remaining_threshold;

        // Signature verification should not be valid until all proofs are submitted
        //
        // Note: In this specific test, each signer has a weight of 1, and the threshold
        // is set to match the total number of signers.
        for (pos, signature) in signatures.iter().enumerate() {
            let proof = merkle_tree.proof(&[pos]);
            assert!(
                sig_verification.verify_signature(signature, proof, SignatureAlwaysValid),
                "signature verification should have worked for a known and valid signature"
            );
            assert!(
                *sig_verification.signature_slots.get(pos).unwrap(),
                "signature slot should be set to true after a successful verification"
            );
            assert_eq!(
                sig_verification.remaining_threshold,
                original_threshold - (pos as u128 + 1),
                "remaining threshold should be decremented after a successful verification"
            );
            if sig_verification.remaining_threshold > 0 {
                assert!(
                    !sig_verification.is_valid(),
                    "signature verification should not be valid if there's any remaining threshold"
                );
            }
        }
        assert_eq!(
            sig_verification.remaining_threshold, 0,
            "remaining threshold should be equal to zero after successfully validating all required signatures"
        );
        assert!(
            sig_verification.is_valid(),
            "signature verification should be valid after successfully validating all required signatures"
        );
    }

    #[test]
    fn test_failed_signature_verification() {
        let TestCase {
            mut sig_verification,
            signatures,
            merkle_tree,
        } = setup::<MAX_SIGNATURES>();

        // Signature verification should not be valid until all proofs are submitted.
        for (pos, signature) in signatures.iter().enumerate() {
            let proof = merkle_tree.proof(&[pos]);
            assert!(
                !sig_verification.verify_signature(
                    signature,
                    proof,
                    SignatureAlwaysInvalid
                ),
                "signature verification should have failed for a known signature that doesn't pass verification"
            );
        }
        assert_unchanged(sig_verification);
    }

    #[test]
    fn test_resistant_to_repeated_signature_submission() {
        let TestCase {
            mut sig_verification,
            signatures,
            merkle_tree,
        } = setup::<MAX_SIGNATURES>();

        let mut submit_first_signature = || {
            sig_verification.verify_signature(
                &signatures[0],
                merkle_tree.proof(&[0]),
                SignatureAlwaysValid,
            )
        };

        // Submit the first signature, which succeeds.
        assert!(
            submit_first_signature(),
            "signature verification should have worked for a known and valid signature"
        );

        // Submit the first signature again, which should fail.
        assert!(
            !submit_first_signature(),
            "signature verification should have failed for an already validated signature"
        );

        // We haven't validated all required signatures, so it should still be invalid.
        assert!(
            !sig_verification.is_valid(),
            "signature verification should still not be valid while there's remaining threshold"
        );

        // Only the first slot should be set.
        let mut slot_iter = sig_verification.signature_slots.iter();
        assert!(
            slot_iter.next().expect("there should be a first slot"),
            "the first signature slot should be set"
        );
        assert!(
            slot_iter.all(|x| !x),
            "remaining signature slots should all unset after failed verification attempts"
        );

        // We successfully validated one signature, so the threshold should be
        // decremented by one.
        assert_eq!(
            sig_verification.remaining_threshold,
            sig_verification.original_threshold - 1,
            "remaining threshold should not change after failed verification attempts"
        )
    }

    /// Checks if inclusion proofs from unrelated trees fail to verify.
    fn resistant_to_invalid_proofs<const NODE_COUNT_A: usize, const NODE_COUNT_B: usize>() {
        let TestCase {
            mut sig_verification,
            signatures,
            ..
        } = setup::<NODE_COUNT_A>();

        // Take a proof from an unrelated merkle tree with a different size.
        let TestCase {
            merkle_tree: unrelated_tree,
            ..
        } = setup::<NODE_COUNT_B>();
        let invalid_proof = unrelated_tree.proof(&[0]);

        assert!(
            !sig_verification
                .verify_signature(&signatures[0], invalid_proof, SignatureAlwaysValid,),
            "signature verification should have failed for an invalid proof"
        );
        assert_unchanged(sig_verification);
    }

    #[test]
    fn test_resistant_to_invalid_proofs_different_node_count() {
        // Same size
        resistant_to_invalid_proofs::<MAX_SIGNATURES, MAX_SIGNATURES>();

        // Smaller tree
        resistant_to_invalid_proofs::<MAX_SIGNATURES, 40>();

        // Bigger tree
        resistant_to_invalid_proofs::<40, MAX_SIGNATURES>();
    }

    #[test]
    fn test_resistant_to_tampered_signatures() {
        let TestCase {
            mut sig_verification,
            merkle_tree,
            ..
        } = setup::<MAX_SIGNATURES>();

        // This is a valid inclusion proof for the first signature.
        let proof = merkle_tree.proof(&[0]);

        // Use a different signature, unrelated to the proof above.
        let TestCase {
            signatures: other_signatures,
            ..
        } = setup::<MAX_SIGNATURES>();

        assert!(
            !sig_verification.verify_signature(&other_signatures[0], proof, SignatureAlwaysValid,),
            "signature verification should have failed for a tampered signature"
        );
        assert_unchanged(sig_verification)
    }

    #[test]
    fn test_serialization() {
        let TestCase {
            mut sig_verification,
            signatures,
            merkle_tree,
        } = setup::<MAX_SIGNATURES>();
        let original_threshold = sig_verification.original_threshold;

        // Submit signatures at random just to create some confusion.
        for (pos, signature) in signatures.iter().enumerate() {
            if signature.signature_bytes[0] & 1 == 0 {
                continue;
            }
            let proof = merkle_tree.proof(&[pos]);
            assert!(
                sig_verification.verify_signature(signature, proof, SignatureAlwaysValid,),
                "signature verification should have worked for a known and valid signature"
            );
        }

        let serialized = sig_verification.serialize();
        let mut deserialized = SignatureVerification::deserialize(&serialized);

        // HACK: Since this is a test field, it isn't serialized; so we need to set it
        // manually for the assertion below to work.
        deserialized.original_threshold = original_threshold;

        assert_eq!(
            sig_verification, deserialized,
            "deserialized data should match the original"
        );
    }

    #[test]
    fn test_batch_context_serialization() {
        // Set up test data for each field in BatchContext
        let test_gateway_root_pda = Pubkey::new_unique();
        let test_domain_separator = random_bytes();
        let test_threshold = u128::from_le_bytes(random_bytes());
        let test_message_hash = random_bytes();
        let test_signer_count = random_bytes::<1>()[0];
        let test_padding = [0u8; 15];

        let batch_context = BatchContext {
            gateway_root_pda: test_gateway_root_pda,
            domain_separator: test_domain_separator,
            threshold: test_threshold,
            message_hash: test_message_hash,
            signer_count: test_signer_count,
            _pad: test_padding,
        };

        // Serialize context into byte array
        let mut bytes = [0u8; size_of::<BatchContext>()];
        batch_context.serialize_into(&mut bytes);

        // Deserialize byte array back into a BatchContext instance
        let deserialized = BatchContext::deserialize(&bytes);

        assert_eq!(batch_context, deserialized)
    }
}
