use std::collections::BTreeMap;

use bnum::types::U256 as BnumU256;
use rand::distributions::Alphanumeric;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::tests::signing_key;
use crate::types::*;

pub(crate) fn random_bytes<const N: usize>(rng: &mut ThreadRng) -> [u8; N] {
    let mut bytes = [0u8; N];
    rng.fill(&mut bytes[..]);
    bytes
}

pub(crate) fn random_string(rng: &mut ThreadRng, len: usize) -> String {
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}

pub(crate) fn random_u256(rng: &mut ThreadRng) -> U256 {
    U256::from_le(random_bytes(rng))
}

pub(crate) fn random_public_key(rng: &mut ThreadRng) -> PublicKey {
    signing_key::random_keypair(rng).1
}

pub(crate) fn random_cross_chain_id(rng: &mut ThreadRng) -> CrossChainId {
    CrossChainId::new(random_string(rng, 10), random_string(rng, 10))
}

pub(crate) fn random_message(rng: &mut ThreadRng) -> Message {
    Message::new(
        random_cross_chain_id(rng),
        random_string(rng, 64),
        random_string(rng, 10),
        random_string(rng, 64),
        random_bytes(rng),
    )
}

pub(crate) fn random_signer(rng: &mut ThreadRng) -> Signer {
    Signer::new(
        random_string(rng, 10),
        random_public_key(rng),
        random_u256(rng),
    )
}

pub(crate) fn random_verifier_set(rng: &mut ThreadRng) -> VerifierSet {
    let mut signers = BTreeMap::new();
    let num_signers = rng.gen_range(1..10);
    for _ in 0..num_signers {
        signers.insert(random_string(rng, 10), random_signer(rng));
    }
    VerifierSet::new(rng.gen(), signers, random_u256(rng))
}

pub(crate) fn random_proof(rng: &mut ThreadRng, message: &[u8]) -> Proof {
    let num_signatures = rng.gen_range(1..10);
    let signatures = (0..num_signatures)
        .map(|_| random_valid_weighted_signature(rng, message))
        .collect();
    Proof::new(signatures, random_u256(rng), rng.gen())
}

pub(crate) fn random_valid_proof_and_message<const MESSAGE_LENGTH: usize>(
    rng: &mut ThreadRng,
) -> (Proof, [u8; MESSAGE_LENGTH]) {
    let message = random_bytes::<MESSAGE_LENGTH>(rng);

    let mut signatures = vec![];
    let mut threshold = BnumU256::ZERO;

    let num_signatures = rng.gen_range(1..10);
    for _ in 0..num_signatures {
        let weighted_signature = random_valid_weighted_signature(rng, &message);

        let weight: BnumU256 = weighted_signature.weight.into();
        dbg!(weight);

        threshold = threshold
            .checked_add(weighted_signature.weight.into())
            .expect("no overflow");
        signatures.push(weighted_signature);
    }

    (Proof::new(signatures, threshold.into(), rng.gen()), message)
}

pub(crate) fn random_valid_weighted_signature(
    rng: &mut ThreadRng,
    message: &[u8],
) -> WeightedSignature {
    // Don't use high weigths in tests to avoid overflows
    let weight = {
        let mut weight_buffer = [0u8; 32];
        weight_buffer[0] = rng.gen();
        U256::from_le(weight_buffer)
    };
    let (signing_key, pubkey) = signing_key::random_keypair(rng);
    let signature = signing_key.sign(message);
    WeightedSignature::new(pubkey, signature, weight)
}

pub(crate) fn random_payload(rng: &mut ThreadRng) -> Payload {
    if rng.gen_bool(0.5) {
        Payload::new_messages(
            (0..rng.gen_range(1..10))
                .map(|_| random_message(rng))
                .collect(),
        )
    } else {
        Payload::new_verifier_set(random_verifier_set(rng))
    }
}

pub(crate) fn random_execute_data(rng: &mut ThreadRng) -> ExecuteData {
    let payload = random_payload(rng);
    let verifier_set = random_verifier_set(rng);
    let domain_separator = random_bytes::<32>(rng);
    let payload_hash = crate::hash_payload(&domain_separator, &verifier_set, &payload);
    let proof = random_proof(rng, &payload_hash);
    ExecuteData::new(payload, proof)
}
