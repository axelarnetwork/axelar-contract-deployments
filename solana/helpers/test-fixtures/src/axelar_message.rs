use axelar_message_primitives::DataPayload;
use axelar_rkyv_encoding::test_fixtures::random_message_with_destination_and_payload;
use axelar_rkyv_encoding::types::{Message, VerifierSet};
use solana_sdk::pubkey::Pubkey;

use crate::execute_data::TestSigner;

pub fn custom_message(destination_address: Pubkey, payload: &DataPayload<'_>) -> Message {
    let payload_hash = payload
        .hash()
        .expect("failed to get payload hash from DataPayload")
        .0;

    random_message_with_destination_and_payload(destination_address.to_string(), *payload_hash)
}

pub fn new_signer_set(signers: &[TestSigner], created_at: u64, threshold: u128) -> VerifierSet {
    let signers_btree = signers
        .iter()
        .map(|signer| (signer.public_key, signer.weight))
        .collect();
    VerifierSet::new(created_at, signers_btree, threshold.into())
}
