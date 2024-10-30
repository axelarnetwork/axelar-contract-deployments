use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, OnceLock};

use axelar_rkyv_encoding::hasher::merkle_trait::Merkle;
use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, NativeHasher, SolanaSyscallHasher};
use axelar_rkyv_encoding::test_fixtures::{
    random_verifier_set_and_signing_keys_fixed_size, TestSigningKey,
};
use axelar_rkyv_encoding::types::{PublicKey, Signature, VerifierSet, VerifierSetLeafNode};
use axelar_solana_gateway::instructions::InitializeConfig;
use axelar_solana_gateway::state::verifier_set_tracker::VerifierSetHash;
use solana_sdk::pubkey::Pubkey;

use crate::runner::TestRunner;

pub const DOMAIN_SEPARATOR: [u8; 32] = [42; 32];
const DEFAULT_SIGNER_SET_SIZE: usize = 64;
static SIGNER_SET_SIZE_IN_TESTS: OnceLock<usize> = OnceLock::new();

pub fn make_initialize_config(
    initial_signer_sets: &[VerifierSetHash],
) -> InitializeConfig<VerifierSetHash> {
    InitializeConfig {
        domain_separator: DOMAIN_SEPARATOR,
        initial_signer_sets: initial_signer_sets.to_vec(),
        minimum_rotation_delay: 0,
        operator: Pubkey::new_unique(),
        previous_signers_retention: 0u128.into(),
    }
}

pub struct TestSuite {
    pub runner: TestRunner,
    pub gateway_config: InitializeConfig<VerifierSetHash>,
    pub gateway_config_pda: Pubkey,
    pub initial_signer_set: VerifierSet,
    pub signing_keys: Arc<BTreeMap<PublicKey, TestSigningKey>>,
    pub initial_verifier_set_tracker_pda: Pubkey,
    pub verification_inputs_iterator: PartialSignatureVerificationIterator,
}

impl TestSuite {
    pub async fn new() -> Self {
        TestSuiteBuilder::default().build().await
    }

    pub async fn new_with_pending_payloads(payload_roots: &[[u8; 32]]) -> Self {
        TestSuiteBuilder::default()
            .with_pending_payloads(payload_roots)
            .build()
            .await
    }
}

#[derive(Default)]
struct TestSuiteBuilder {
    initial_payload_verification_sessions: Vec<[u8; 32]>,
}

impl TestSuiteBuilder {
    fn with_pending_payloads(mut self, payload_roots: &[[u8; 32]]) -> Self {
        self.initial_payload_verification_sessions = payload_roots.to_vec();
        self
    }

    async fn build(self) -> TestSuite {
        let mut runner = TestRunner::new().await;

        let num_signers = SIGNER_SET_SIZE_IN_TESTS.get_or_init(|| {
            std::env::var("SIGNER_SET_SIZE_IN_TESTS")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(DEFAULT_SIGNER_SET_SIZE)
        });

        let (initial_signer_set, signing_keys) =
            random_verifier_set_and_signing_keys_fixed_size(*num_signers, DOMAIN_SEPARATOR);
        let signing_keys = Arc::new(signing_keys);

        let initial_signer_set_root =
            Merkle::<NativeHasher>::calculate_merkle_root(&initial_signer_set)
                .expect("expected a non-empty signer set");

        let gateway_config = make_initialize_config(&[initial_signer_set_root]);

        let gateway_config_pda = runner
            .initialize_gateway_config_account(gateway_config.clone())
            .await;

        let (initial_verifier_set_tracker_pda, _bump) =
            axelar_solana_gateway::get_verifier_set_tracker_pda(
                &axelar_solana_gateway::ID,
                initial_signer_set_root,
            );

        for payload_merkle_root in self.initial_payload_verification_sessions {
            runner
                .initialize_payload_verification_session(gateway_config_pda, payload_merkle_root)
                .await;
        }

        let verification_inputs_iterator = PartialSignatureVerificationIterator::new(
            &initial_signer_set,
            Arc::clone(&signing_keys),
        );

        TestSuite {
            runner,
            gateway_config,
            gateway_config_pda,
            initial_signer_set,
            signing_keys,
            initial_verifier_set_tracker_pda,
            verification_inputs_iterator,
        }
    }
}

/// All the required inputs to submit a `VerifySignature` instruction
pub struct SignatureVerificationInput {
    pub leaf: VerifierSetLeafNode<SolanaSyscallHasher>,
    pub proof: MerkleProof<SolanaSyscallHasher>,
    pub signature: Signature,
}

/// Produces iterators of inputs for the `VerifySignature` instruction, given a
/// payload merkle root.
pub struct PartialSignatureVerificationIterator {
    leaves: Vec<VerifierSetLeafNode<SolanaSyscallHasher>>,
    proofs: Vec<MerkleProof<SolanaSyscallHasher>>,
    signing_keys: Arc<BTreeMap<PublicKey, TestSigningKey>>,
}

impl PartialSignatureVerificationIterator {
    fn new(
        verifier_set: &VerifierSet,
        signing_keys: Arc<BTreeMap<PublicKey, TestSigningKey>>,
    ) -> Self {
        let leaves: Vec<VerifierSetLeafNode<SolanaSyscallHasher>> =
            verifier_set.merkle_leaves().collect();
        let proofs: Vec<MerkleProof<SolanaSyscallHasher>> = verifier_set.merkle_proofs().collect();
        Self {
            leaves,
            proofs,
            signing_keys,
        }
    }

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
    signing_keys: Arc<BTreeMap<PublicKey, TestSigningKey>>,
    payload_merkle_root: [u8; 32],
}

impl<'a> Iterator for SignatureVerificationIterator<'a> {
    type Item = SignatureVerificationInput;

    fn next(&mut self) -> Option<Self::Item> {
        let proof = {
            let proof = self.proofs.pop_front()?;
            // `MerkleProof` doesn't implement `Clone`.
            let hashes = proof.proof_hashes().to_vec();
            MerkleProof::new(hashes)
        };
        let leaf = self.leaves.pop_front()?;
        let signature = self
            .signing_keys
            .get(&leaf.signer_pubkey)
            .expect("missing signing keys for signer")
            .sign(&self.payload_merkle_root);
        Some(SignatureVerificationInput {
            leaf: *leaf,
            proof,
            signature,
        })
    }
}
