pub mod signing_key;

use std::collections::BTreeMap;

use bnum::types::U128 as BnumU128;
use rand::distributions::Alphanumeric;
use rand::rngs::OsRng;
use rand::Rng;
pub use signing_key::{
    random_ecdsa_keypair, random_ed25519_keypair, random_keypair, TestSigningKey,
};
pub use {ed25519_dalek, libsecp256k1};

use crate::hash_payload;
use crate::hasher::solana::{self, SolanaKeccak256Hasher};
use crate::types::*;

pub fn test_hasher_impl<'a>() -> solana::SolanaKeccak256Hasher<'a> {
    SolanaKeccak256Hasher::default()
}

pub fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    OsRng.fill(&mut bytes[..]);
    bytes
}

pub fn random_string(len: usize) -> String {
    OsRng
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub fn random_u128() -> U128 {
    U128::from_le(random_bytes())
}

pub fn random_public_key() -> PublicKey {
    signing_key::random_keypair().1
}

pub fn random_cross_chain_id() -> CrossChainId {
    CrossChainId::new(random_string(10), random_string(10))
}

pub fn random_message() -> Message {
    Message::new(
        random_cross_chain_id(),
        random_string(64),
        random_string(10),
        random_string(64),
        random_bytes(),
        random_bytes(),
    )
}

pub fn random_message_with_destination_and_payload(
    destination_address: String,
    payload_hash: [u8; 32],
) -> Message {
    let mut message = random_message();
    message.destination_address = destination_address;
    message.payload_hash = payload_hash;
    message
}

pub fn random_valid_verifier_set_fixed_size(num_signers: usize) -> VerifierSet {
    let mut signers = BTreeMap::new();
    let mut total_weight = BnumU128::ZERO;
    for _ in 0..num_signers {
        let pubkey = random_public_key();
        let weight = random_weight();
        total_weight += Into::<BnumU128>::into(weight);
        signers.insert(pubkey, weight);
    }
    VerifierSet::new(OsRng.gen(), signers, total_weight.into(), random_bytes())
}

pub fn random_valid_verifier_set() -> VerifierSet {
    let num_signers = OsRng.gen_range(1..10);
    random_valid_verifier_set_fixed_size(num_signers)
}

pub fn random_proof(message: &[u8]) -> Proof {
    let num_signatures = OsRng.gen_range(1..10);
    let signatures = (0..num_signatures)
        .map(|_| random_valid_weighted_signature(message))
        .collect();
    Proof::new(signatures, random_u128(), OsRng.gen())
}

pub fn random_valid_proof_and_message<const MESSAGE_LENGTH: usize>() -> (Proof, [u8; MESSAGE_LENGTH])
{
    let (proof, message, ..) = random_valid_proof_message_and_verifier_set();
    (proof, message)
}

pub fn random_valid_proof_and_verifier_set(message: &[u8]) -> (Proof, VerifierSet) {
    let nonce: u64 = OsRng.gen();
    let num_signatures = OsRng.gen_range(1..10);
    let domain_separator = random_bytes();

    // Generate signatures and calculate the total weight.
    let mut threshold = BnumU128::ZERO;
    let mut signatures_by_signer = BTreeMap::new();
    for _ in 0..num_signatures {
        let (pubkey, weighted_signature) = random_valid_weighted_signature(message);
        threshold = threshold
            .checked_add(weighted_signature.weight.into())
            .expect("no overflow");
        signatures_by_signer.insert(pubkey, weighted_signature);
    }

    // Build internal signatures/signer values, ordered by public key
    let signatures = signatures_by_signer.clone();
    let verifier_set_signers = signatures_by_signer
        .iter()
        .map(|(pubkey, weighted_signature)| (*pubkey, weighted_signature.weight))
        .collect();

    let verifier_set = VerifierSet::new(
        nonce,
        verifier_set_signers,
        threshold.into(),
        domain_separator,
    );
    let proof = Proof::new(signatures, threshold.into(), nonce);

    // Confidence checks
    assert_eq!(verifier_set.quorum, proof.threshold);
    assert_eq!(
        verifier_set.signers.len(),
        proof.signers_with_signatures.len()
    );
    assert_eq!(verifier_set.created_at, proof.nonce);

    let proof_pubkeys = proof.signers_with_signatures.keys();
    let verifier_set_pubkeys = verifier_set.signers.keys();
    proof_pubkeys
        .zip(verifier_set_pubkeys)
        .for_each(|(a, b)| assert_eq!(a, b));

    (proof, verifier_set)
}

pub fn random_valid_proof_message_and_verifier_set<const MESSAGE_LENGTH: usize>(
) -> (Proof, [u8; MESSAGE_LENGTH], VerifierSet) {
    let message = random_bytes::<MESSAGE_LENGTH>();
    let (proof, verifier_set) = random_valid_proof_and_verifier_set(&message);
    (proof, message, verifier_set)
}

pub fn random_valid_weighted_signature(message: &[u8]) -> (PublicKey, WeightedSigner) {
    let weight = random_weight();
    let (signing_key, pubkey) = signing_key::random_keypair();
    let signature = signing_key.sign(message);
    (pubkey, WeightedSigner::new(Some(signature), weight))
}

/// Generates a weight between 0 and 255.
pub fn random_weight() -> U128 {
    let mut weight_buffer = [0u8; 16];
    weight_buffer[0] = OsRng.gen();
    U128::from_le(weight_buffer)
}

pub fn random_messages() -> Vec<Message> {
    (0..OsRng.gen_range(1..10))
        .map(|_| random_message())
        .collect()
}

pub fn random_payload() -> Payload {
    if OsRng.gen_bool(0.5) {
        Payload::new_messages(random_messages())
    } else {
        Payload::new_verifier_set(random_valid_verifier_set())
    }
}

pub fn random_execute_data() -> ExecuteData {
    let payload = random_payload();
    let verifier_set = random_valid_verifier_set();
    let domain_separator = random_bytes::<32>();
    let payload_hash = crate::hash_payload(
        &domain_separator,
        &verifier_set,
        &payload,
        test_hasher_impl(),
    );
    let proof = random_proof(&payload_hash);
    ExecuteData::new(payload, proof)
}

pub fn random_verifier_set_and_signing_keys_fixed_size(
    num_signers: usize,
    domain_separator: [u8; 32],
) -> (VerifierSet, BTreeMap<PublicKey, TestSigningKey>) {
    let mut signers = BTreeMap::new();
    let mut signing_keys = BTreeMap::new();
    let mut total_weight = BnumU128::ZERO;
    for _ in 0..num_signers {
        let (signing_key, public_key) = random_keypair();
        let weight = random_weight();
        total_weight += Into::<BnumU128>::into(weight);
        signers.insert(public_key, weight);
        signing_keys.insert(public_key, signing_key);
    }
    let verifier_set =
        VerifierSet::new(OsRng.gen(), signers, total_weight.into(), domain_separator);
    (verifier_set, signing_keys)
}

pub fn random_verifier_set_and_signing_keys(
    domain_separator: [u8; 32],
) -> (VerifierSet, BTreeMap<PublicKey, TestSigningKey>) {
    let num_signers = OsRng.gen_range(1..10);
    random_verifier_set_and_signing_keys_fixed_size(num_signers, domain_separator)
}

pub fn random_valid_execute_data_and_verifier_set_for_payload(
    domain_separator: [u8; 32],
    payload: Payload,
) -> (ExecuteData, VerifierSet) {
    let (verifier_set, signing_keys) = random_verifier_set_and_signing_keys(domain_separator);
    let original_payload_hash = hash_payload(
        &domain_separator,
        &verifier_set,
        &payload,
        test_hasher_impl(),
    );

    let weighted_signatures = signing_keys
        .iter()
        .map(|(pubkey, signing_key)| {
            let signature = signing_key.sign(&original_payload_hash);
            let weight = verifier_set.signers.get(pubkey).unwrap();
            (*pubkey, WeightedSigner::new(Some(signature), *weight))
        })
        .collect();

    let proof = Proof::new(
        weighted_signatures,
        verifier_set.quorum,
        verifier_set.created_at,
    );

    let execute_data = ExecuteData::new(payload, proof);

    (execute_data, verifier_set)
}

pub fn random_valid_execute_data_and_verifier_set(
    domain_separator: [u8; 32],
) -> (ExecuteData, VerifierSet) {
    let payload = random_payload();
    random_valid_execute_data_and_verifier_set_for_payload(domain_separator, payload)
}

pub fn random_execute_data_and_verifier_set_for_payload_with_invalid_signatures(
    domain_separator: [u8; 32],
    payload: Payload,
) -> (ExecuteData, VerifierSet) {
    let (mut execute_data, verifier_set) =
        random_valid_execute_data_and_verifier_set_for_payload(domain_separator, payload);

    // Flip a bit in the first byte of the signature
    let signature_bytes: &mut [u8] = match &mut execute_data
        .proof
        .signers_with_signatures
        .values_mut()
        .next()
        .unwrap()
        .signature
    {
        Some(Signature::EcdsaRecoverable(bytes)) => bytes.as_mut_slice(),
        Some(Signature::Ed25519(bytes)) => bytes.as_mut_slice(),
        _ => unimplemented!("signature not attached"),
    };
    signature_bytes[0] ^= 1;

    (execute_data, verifier_set)
}
