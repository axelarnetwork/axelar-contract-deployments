use axelar_rkyv_encoding::types::{
    ArchivedExecuteData, ExecuteData, Payload, VerifierSet, WeightedSigner,
};
use axelar_rkyv_encoding::{encode, hash_payload};
use gateway::hasher_impl;

use crate::test_setup::SigningVerifierSet;
pub use crate::test_signer::TestSigner;

pub fn prepare_execute_data(
    payload: Payload,
    signers: &SigningVerifierSet,
    domain_separator: &[u8; 32],
) -> (Vec<u8>, VerifierSet) {
    // Setup
    let verifier_set = signers.verifier_set();
    let payload_hash = hash_payload(domain_separator, &verifier_set, &payload, hasher_impl());

    // Iterating over a btree results in a sorted vector
    let weighted_signatures: Vec<_> = signers
        .signers
        .iter()
        .map(|signer| {
            let signature = signer.secret_key.sign(&payload_hash);
            let weight = signer.weight;
            (
                signer.public_key,
                WeightedSigner::new(Some(signature), weight),
            )
        })
        .collect();

    // Do as the 'multisig_prover' contract would
    let execute_data_bytes = encode::<0>(
        verifier_set.created_at(),
        *verifier_set.quorum(),
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
