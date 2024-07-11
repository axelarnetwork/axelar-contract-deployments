use std::collections::BTreeMap;

use axelar_rkyv_encoding::types::{
    ArchivedExecuteData, ExecuteData, Payload, VerifierSet, WeightedSigner,
};
use axelar_rkyv_encoding::{encode, hash_payload};

pub use crate::test_signer::TestSigner;

pub fn prepare_execute_data(
    payload: Payload,
    test_signers: &[TestSigner],
    threshold: u128,
    nonce: u64,
    domain_separator: &[u8; 32],
) -> (Vec<u8>, VerifierSet) {
    // Setup
    let mut signers = BTreeMap::new();
    let mut signing_keys = BTreeMap::new();
    for signer in test_signers {
        signers.insert(signer.public_key, signer.weight);
        signing_keys.insert(signer.public_key, &signer.secret_key);
    }

    let verifier_set = VerifierSet::new(nonce, signers, threshold.into());

    let payload_hash = hash_payload(domain_separator, &verifier_set, &payload);

    // Iterating over a btree results in a sorted vector
    let weighted_signatures: Vec<_> = signing_keys
        .iter()
        .map(|(pubkey, signing_key)| {
            let signature = signing_key.sign(&payload_hash);
            let weight = verifier_set.signers().get(pubkey).unwrap();
            (*pubkey, WeightedSigner::new(Some(signature), *weight))
        })
        .collect();

    // Do as the 'multisig_prover' contract would
    let execute_data_bytes = encode::<0>(
        verifier_set.created_at(),
        *verifier_set.threshold(),
        weighted_signatures,
        payload,
    )
    .unwrap();

    // Confidence check: ExecuteData can be deserialized
    ExecuteData::from_bytes(&execute_data_bytes).expect("valid deserialization");

    // Confidence check: ExecuteData can be cast to its archive
    ArchivedExecuteData::from_bytes(&execute_data_bytes).expect("valid archival");

    (execute_data_bytes, verifier_set)
}
