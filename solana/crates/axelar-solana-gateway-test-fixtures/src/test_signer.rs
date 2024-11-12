//! Verifier set utilities that provide ability to sign over messages

use std::collections::VecDeque;
use std::sync::Arc;

use axelar_rkyv_encoding::hasher::merkle_trait::Merkle;
use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, SolanaSyscallHasher};
use axelar_rkyv_encoding::test_fixtures::{random_ecdsa_keypair, TestSigningKey};
use axelar_rkyv_encoding::types::{PublicKey, Signature, VerifierSet, VerifierSetLeafNode, U128};
use solana_sdk::pubkey::Pubkey;

/// Uitility verifier set representation that has access to the signing keys
#[derive(Clone, Debug)]
pub struct SigningVerifierSet {
    /// signers that have access to the given verifier set
    pub signers: Arc<[TestSigner]>,
    /// the nonce for the verifier set
    pub nonce: u64,
    /// quorum for the verifier set
    pub quorum: U128,
    /// the domain separator for the verifier set
    pub domain_separator: [u8; 32],
}

impl SigningVerifierSet {
    /// Create a new `SigningVerifierSet`
    ///
    /// # Panics
    /// if the calculated quorum is larger than u128
    pub fn new(signers: Arc<[TestSigner]>, nonce: u64, domain_separator: [u8; 32]) -> Self {
        let quorum = signers
            .iter()
            .map(|signer| signer.weight)
            .try_fold(U128::ZERO, U128::checked_add)
            .expect("no arithmetic overflow");
        Self::new_with_quorum(signers, nonce, quorum, domain_separator)
    }

    /// Create a new `SigningVerifierSet` with a custom quorum
    #[must_use]
    pub fn new_with_quorum(
        signers: Arc<[TestSigner]>,
        nonce: u64,
        quorum: U128,
        domain_separator: [u8; 32],
    ) -> Self {
        Self {
            signers,
            nonce,
            quorum,
            domain_separator,
        }
    }

    /// Get the verifier set tracket PDA and bump
    #[must_use]
    pub fn verifier_set_tracker(&self) -> (Pubkey, u8) {
        axelar_solana_gateway::get_verifier_set_tracker_pda(self.verifier_set().hash_with_merkle())
    }

    /// Transform into the verifier set that the gateway expects to operate on
    #[must_use]
    pub fn verifier_set(&self) -> VerifierSet {
        let signers = self
            .signers
            .iter()
            .map(|x| (x.public_key, x.weight))
            .collect();
        VerifierSet::new(self.nonce, signers, self.quorum, self.domain_separator)
    }

    /// Start a new signing session using the signers on this struct and the
    /// verifier set that's expected to be part of the merklesied data. This
    /// allows us to generate signature iterators where not all of the signers
    /// participate.
    #[must_use]
    pub fn init_signing_session(
        &self,
        verifier_set: &VerifierSet,
    ) -> PartialSignatureVerificationIterator {
        PartialSignatureVerificationIterator::new(Arc::clone(&self.signers), verifier_set)
    }
}

#[cfg(test)]
mod tests {
    use axelar_rkyv_encoding::hasher::merkle_tree::{Hasher, NativeHasher};

    use super::*;
    use crate::gateway::make_verifier_set;

    #[test]
    fn test_can_reconstruct_payload_hash() {
        let verifier_set = make_verifier_set(&[22, 32], 555, [42; 32]);
        let verifier_set_hash = verifier_set.verifier_set().hash_with_merkle();
        let expected_payload_hash = verifier_set.verifier_set().payload_hash();

        let re_derived_payload_hash =
            solana_sdk::keccak::hashv(&[VerifierSet::HASH_PREFIX, &verifier_set_hash]).0;
        let re_derived_payload_hash = NativeHasher::concat_and_hash(&re_derived_payload_hash, None);
        assert_eq!(
            expected_payload_hash, re_derived_payload_hash,
            "hashes not equal"
        );
    }
}

/// Single test signer
#[derive(Clone, Debug)]
pub struct TestSigner {
    /// public key
    pub public_key: PublicKey,
    /// privaet key
    pub secret_key: TestSigningKey,
    /// associated weight
    pub weight: U128,
}

/// Create a new signer with the given wetight
#[must_use]
pub fn create_signer_with_weight(weight: u128) -> TestSigner {
    let (secret_key, public_key) = random_ecdsa_keypair();

    TestSigner {
        public_key,
        secret_key,
        weight: weight.into(),
    }
}

/// All the required inputs to submit a `VerifySignature` instruction
pub struct SignatureVerificationInput {
    /// leaf node for a signature verification
    pub verifier_set_leaf: VerifierSetLeafNode<SolanaSyscallHasher>,
    /// merkple proof for the given leaf node
    pub verifier_set_proof: MerkleProof<SolanaSyscallHasher>,
    /// the raw signature
    pub signature: Signature,
}

/// Produces iterators of inputs for the `VerifySignature` instruction, given a
/// payload merkle root.
pub struct PartialSignatureVerificationIterator {
    leaves: Vec<VerifierSetLeafNode<SolanaSyscallHasher>>,
    proofs: Vec<MerkleProof<SolanaSyscallHasher>>,
    signing_keys: Arc<[TestSigner]>,
}

impl PartialSignatureVerificationIterator {
    fn new(signers: Arc<[TestSigner]>, verifier_set: &VerifierSet) -> Self {
        let leaves: Vec<VerifierSetLeafNode<SolanaSyscallHasher>> =
            verifier_set.merkle_leaves().collect();
        let proofs: Vec<MerkleProof<SolanaSyscallHasher>> = verifier_set.merkle_proofs().collect();
        Self {
            leaves,
            proofs,
            signing_keys: signers,
        }
    }

    /// Create a signature iterator for a given payload root
    pub fn for_payload_root(
        &self,
        payload_merkle_root: [u8; 32],
    ) -> impl Iterator<Item = SignatureVerificationInput> + '_ {
        let leaves = VecDeque::from_iter(&self.leaves);
        let proofs = VecDeque::from_iter(&self.proofs);
        SignatureVerificationIterator {
            leaves,
            proofs,
            signing_keys: Arc::clone(&self.signing_keys),
            payload_merkle_root,
        }
    }
}

struct SignatureVerificationIterator<'a> {
    leaves: VecDeque<&'a VerifierSetLeafNode<SolanaSyscallHasher>>,
    proofs: VecDeque<&'a MerkleProof<SolanaSyscallHasher>>,
    signing_keys: Arc<[TestSigner]>,
    payload_merkle_root: [u8; 32],
}

impl<'a> Iterator for SignatureVerificationIterator<'a> {
    type Item = SignatureVerificationInput;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let proof = {
                let proof = self.proofs.pop_front()?;
                // `MerkleProof` doesn't implement `Clone`.
                let hashes = proof.proof_hashes().to_vec();
                MerkleProof::new(hashes)
            };
            let leaf = self.leaves.pop_front()?;
            if let Some(signature) = self
                .signing_keys
                .iter()
                .find(|x| x.public_key == leaf.signer_pubkey)
                .map(|x| x.secret_key.sign(&self.payload_merkle_root))
            {
                // if we have a signer for the given signer set then we sign the root & return
                // it
                return Some(SignatureVerificationInput {
                    verifier_set_leaf: *leaf,
                    verifier_set_proof: proof,
                    signature,
                });
            }
        }
    }
}
