use crate::hash_payload;
use crate::test_fixtures::{
    random_bytes, random_execute_data, random_payload, random_valid_execute_data_and_verifier_set,
    random_valid_verifier_set, test_hasher_impl,
};
use crate::types::*;

#[test]
fn test_hash_payload() {
    let domain_separator: [u8; 32] = random_bytes();
    let verifier_set = random_valid_verifier_set();
    let payload = random_payload();

    let hash1 = hash_payload(
        &domain_separator,
        &verifier_set,
        &payload,
        test_hasher_impl(),
    );

    // Re-hash the same inputs and verify the hash is the same
    let hash2 = hash_payload(
        &domain_separator,
        &verifier_set,
        &payload,
        test_hasher_impl(),
    );
    assert_eq!(hash1, hash2);

    // Hash with different inputs and verify the hash is different
    let different_payload = random_payload();
    let different_hash = hash_payload(
        &domain_separator,
        &verifier_set,
        &different_payload,
        test_hasher_impl(),
    );
    assert_ne!(hash1, different_hash);

    // Hash with a different domain separator and verify the hash is different
    let different_domain_separator: [u8; 32] = random_bytes();
    let different_domain_hash = hash_payload(
        &different_domain_separator,
        &verifier_set,
        &payload,
        test_hasher_impl(),
    );
    assert_ne!(hash1, different_domain_hash);
}

#[test]
fn consistent_payload_hashes_across_boundaries() {
    // Setup
    let execute_data = random_execute_data();
    let verifier_set = random_valid_verifier_set();
    let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
    let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };
    let domain_separator = random_bytes::<32>();

    // Create the external and internal hashes
    let external_hash = hash_payload(
        &domain_separator,
        &verifier_set,
        &execute_data.payload,
        test_hasher_impl(),
    );
    let internal_hash = archived.hash_payload_for_verifier_set(
        &domain_separator,
        &verifier_set,
        test_hasher_impl(),
    );

    // Compare
    assert_eq!(external_hash, internal_hash);
}

#[test]
fn internal_and_external_payload_hash_equivalence() {
    // Setup

    let domain_separator = random_bytes::<32>();
    let (execute_data, verifier_set) =
        random_valid_execute_data_and_verifier_set(&domain_separator);

    // Hash the payload as the multisig-prover contract would
    let payload_hash = hash_payload(
        &domain_separator,
        &verifier_set,
        &execute_data.payload,
        test_hasher_impl(),
    );

    let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
    let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };

    // Produce a hash as the Gateway program would
    let internal_hash = archived.internal_payload_hash(&domain_separator, test_hasher_impl());

    // Compare
    assert_eq!(internal_hash, payload_hash, "hashes must match");
    assert!(archived.proof.validate_for_message(&payload_hash).is_ok()); // Confidence check
}
