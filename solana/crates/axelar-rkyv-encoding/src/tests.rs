#![cfg(test)]

pub(crate) mod fixtures;
mod signing_key;

use crate::hash_payload;
use crate::tests::fixtures::{
    random_bytes, random_execute_data, random_payload, random_verifier_set,
};
use crate::types::*;

#[test]
fn test_hash_payload() {
    let mut rng = rand::thread_rng();

    let domain_separator: [u8; 32] = random_bytes(&mut rng);
    let verifier_set = random_verifier_set(&mut rng);
    let payload = random_payload(&mut rng);

    let hash1 = hash_payload(&domain_separator, &verifier_set, &payload);

    // Re-hash the same inputs and verify the hash is the same
    let hash2 = hash_payload(&domain_separator, &verifier_set, &payload);
    assert_eq!(hash1, hash2);

    // Hash with different inputs and verify the hash is different
    let different_payload = random_payload(&mut rng);
    let different_hash = hash_payload(&domain_separator, &verifier_set, &different_payload);
    assert_ne!(hash1, different_hash);

    // Hash with a different domain separator and verify the hash is different
    let different_domain_separator: [u8; 32] = random_bytes(&mut rng);
    let different_domain_hash = hash_payload(&different_domain_separator, &verifier_set, &payload);
    assert_ne!(hash1, different_domain_hash);
}

#[test]
fn consistent_payload_hashes_across_boundaries() {
    // Setup
    let mut rng = rand::thread_rng();
    let execute_data = random_execute_data(&mut rng);
    let verifier_set = random_verifier_set(&mut rng);
    let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
    let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };
    let domain_separator = random_bytes::<32>(&mut rng);

    // Create the external and internal hashes
    let external_hash = hash_payload(&domain_separator, &verifier_set, &execute_data.payload);
    let internal_hash = archived.hash_payload_for_verifier_set(&domain_separator, &verifier_set);

    // Compare
    assert_eq!(external_hash, internal_hash);
}
