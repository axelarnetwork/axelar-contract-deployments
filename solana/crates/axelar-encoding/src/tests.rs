#![cfg(test)]

use std::collections::BTreeMap;

use rand::distributions::Alphanumeric;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::hash_payload;
use crate::types::*;

fn random_bytes<const N: usize>(rng: &mut ThreadRng) -> [u8; N] {
    let mut bytes = [0u8; N];
    rng.fill(&mut bytes[..]);
    bytes
}

fn random_string(rng: &mut ThreadRng, len: usize) -> String {
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

fn random_u256(rng: &mut ThreadRng) -> U256 {
    U256::from_be(random_bytes(rng))
}

fn random_public_key(rng: &mut ThreadRng) -> PublicKey {
    if rng.gen_bool(0.5) {
        PublicKey::new_ecdsa(random_bytes(rng))
    } else {
        PublicKey::new_ed25519(random_bytes(rng))
    }
}

fn random_signature(rng: &mut ThreadRng) -> Signature {
    if rng.gen_bool(0.5) {
        Signature::new_ecdsa_recoverable(random_bytes(rng))
    } else {
        Signature::new_ed25519(random_bytes(rng))
    }
}

fn random_cross_chain_id(rng: &mut ThreadRng) -> CrossChainId {
    CrossChainId::new(random_string(rng, 10), random_string(rng, 10))
}

fn random_message(rng: &mut ThreadRng) -> Message {
    Message::new(
        random_cross_chain_id(rng),
        random_string(rng, 64),
        random_string(rng, 10),
        random_string(rng, 64),
        random_bytes(rng),
    )
}

fn random_signer(rng: &mut ThreadRng) -> Signer {
    Signer::new(
        random_string(rng, 10),
        random_public_key(rng),
        random_u256(rng),
    )
}

fn random_worker_set(rng: &mut ThreadRng) -> WorkerSet {
    let mut signers = BTreeMap::new();
    let num_signers = rng.gen_range(1..10);
    for _ in 0..num_signers {
        signers.insert(random_string(rng, 10), random_signer(rng));
    }
    WorkerSet::new(rng.gen(), signers, random_u256(rng))
}

fn random_weighted_signature(rng: &mut ThreadRng) -> WeightedSignature {
    WeightedSignature::new(
        random_public_key(rng),
        random_signature(rng),
        random_u256(rng),
    )
}

fn random_proof(rng: &mut ThreadRng) -> Proof {
    let num_signatures = rng.gen_range(1..10);
    let signatures = (0..num_signatures)
        .map(|_| random_weighted_signature(rng))
        .collect();
    Proof::new(signatures, random_u256(rng), rng.gen())
}

fn random_payload(rng: &mut ThreadRng) -> Payload {
    if rng.gen_bool(0.5) {
        Payload::new_messages(
            (0..rng.gen_range(1..10))
                .map(|_| random_message(rng))
                .collect(),
        )
    } else {
        Payload::new_worker_set(random_worker_set(rng))
    }
}

fn random_execute_data(rng: &mut ThreadRng) -> ExecuteData {
    ExecuteData::new(random_payload(rng), random_proof(rng))
}

#[test]
fn test_serialize_deserialize_execute_data() {
    let mut rng = rand::thread_rng();
    let execute_data = random_execute_data(&mut rng);

    let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
    let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };

    assert_eq!(*archived, execute_data);
}

#[test]
fn test_hash_payload() {
    let mut rng = rand::thread_rng();

    let domain_separator: [u8; 32] = random_bytes(&mut rng);
    let worker_set = random_worker_set(&mut rng);
    let payload = random_payload(&mut rng);

    let hash1 = hash_payload(&domain_separator, &worker_set, &payload);

    // Re-hash the same inputs and verify the hash is the same
    let hash2 = hash_payload(&domain_separator, &worker_set, &payload);
    assert_eq!(hash1, hash2);

    // Hash with different inputs and verify the hash is different
    let different_payload = random_payload(&mut rng);
    let different_hash = hash_payload(&domain_separator, &worker_set, &different_payload);
    assert_ne!(hash1, different_hash);

    // Hash with a different domain separator and verify the hash is different
    let different_domain_separator: [u8; 32] = random_bytes(&mut rng);
    let different_domain_hash = hash_payload(&different_domain_separator, &worker_set, &payload);
    assert_ne!(hash1, different_domain_hash);
}
