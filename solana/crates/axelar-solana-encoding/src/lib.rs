//! This module provides cryptographic primitives and data structures to support
//! encoding, hashing, and Merkle tree operations on sets of verifiers and
//! payloads.
//!
//! # Overview
//!
//! This code includes functions for encoding and hashing structured data, such
//! as verifier sets and message payloads.
//! It provides Merkle tree-based encoding and proofs for verifiable payloads.
//! The main functions include `encode`, `hash_payload`.
//!
//! # Usage
//!
//! - `encode` encodes `execute_data` components and constructs Merkle proofs
//!   for payloads.
//! - `hash_payload` generates a hash for payload data given a specific domain
//!   and verifier set.

use core::mem::size_of;
use std::collections::BTreeMap;

use error::EncodingError;
use hasher::{NativeHasher, VecBuf};
use rs_merkle::MerkleTree;
use types::payload::Payload;
use types::pubkey::{PublicKey, Signature};
use types::verifier_set::{verifier_set_hash, VerifierSet};
use udigest::encoding::EncodeValue;
pub use {borsh, rs_merkle};

use crate::types::execute_data::{
    ExecuteData, MerkleisedMessage, MerkleisedPayload, SigningVerifierSetInfo,
};
use crate::types::messages::MessageLeaf;
use crate::types::verifier_set::VerifierSetLeaf;

pub mod error;
pub mod hasher;
pub mod types;

/// Encodes `execute_data` components using a custom verifier set, signers, and
/// a domain separator.
///
/// # Errors
/// - IO Error when encoding the data
/// - Verifier Set has too many items in it
/// - Verifier Set has no items in it
/// - Payload messages have too many items in it
/// - Payload messages has no items in t
pub fn encode(
    signing_verifier_set: &VerifierSet,
    signers_with_signatures: &BTreeMap<PublicKey, Signature>,
    domain_separator: [u8; 32],
    payload: Payload,
) -> Result<Vec<u8>, EncodingError> {
    let leaves = types::verifier_set::merkle_tree_leaves(signing_verifier_set, &domain_separator)?
        .collect::<Vec<_>>();
    let signer_merkle_tree = merkle_tree::<NativeHasher, VerifierSetLeaf>(leaves.iter());
    let signing_verifier_set_merkle_root = signer_merkle_tree
        .root()
        .ok_or(EncodingError::CannotMerkeliseEmptyVerifierSet)?;
    let (payload_merkle_root, payload_items) =
        hash_payload_internal(payload, domain_separator, signing_verifier_set_merkle_root)?;

    let signing_verifier_set_leaves = leaves
        .into_iter()
        .filter_map(|leaf| {
            if let Some(signature) = signers_with_signatures.get(&leaf.signer_pubkey) {
                let merkle_proof = signer_merkle_tree.proof(&[leaf.position.into()]);
                return Some(SigningVerifierSetInfo {
                    signature: *signature,
                    leaf,
                    merkle_proof: merkle_proof.to_bytes(),
                });
            }
            None
        })
        .collect::<Vec<_>>();
    let execute_data = ExecuteData {
        signing_verifier_set_merkle_root,
        signing_verifier_set_leaves,
        payload_merkle_root,
        payload_items,
    };
    let capacity = estimate_size(&execute_data);
    let mut buffer = Vec::with_capacity(capacity);
    borsh::to_writer(&mut buffer, &execute_data)?;
    Ok(buffer)
}

fn estimate_size(execute_data: &ExecuteData) -> usize {
    size_of::<ExecuteData>()
        .saturating_add({
            // estimate heap allocations
            match &execute_data.payload_items {
                MerkleisedPayload::VerifierSetRotation { .. } => 0,
                MerkleisedPayload::NewMessages { messages } => {
                    size_of::<MerkleisedMessage>()
                        .saturating_mul(messages.len())
                        .saturating_mul({
                            // allocate for 4 hashes
                            let avg_proof_size = size_of::<[u8; 32]>().saturating_mul(4);
                            // average extra heap allocations by all the Strings in the Message
                            // struct
                            let avg_message_size = 256_usize;
                            avg_message_size.saturating_add(avg_proof_size)
                        })
                }
            }
        })
        .saturating_add(
            size_of::<SigningVerifierSetInfo>()
                .saturating_mul(execute_data.signing_verifier_set_leaves.len()),
        )
}

/// Hashes a payload by constructing a Merkle tree for the verifier set,
/// generating a unique root hash for payload validation.
///
/// # Errors
/// - When the verifier set is empty
/// - When the verifier set is too large
pub fn hash_payload(
    domain_separator: &[u8; 32],
    signer_verifier_set: &VerifierSet,
    payload: Payload,
) -> Result<[u8; 32], EncodingError> {
    let verifier_set_leaves =
        types::verifier_set::merkle_tree_leaves(signer_verifier_set, domain_separator)?
            .collect::<Vec<_>>();
    let mt = merkle_tree::<NativeHasher, VerifierSetLeaf>(verifier_set_leaves.iter());
    let signing_verifier_set_merkle_root = mt
        .root()
        .ok_or(EncodingError::CannotMerkeliseEmptyVerifierSet)?;
    let (payload_hash, _merklesied_payload) =
        hash_payload_internal(payload, *domain_separator, signing_verifier_set_merkle_root)?;

    Ok(payload_hash)
}

/// Internal function for hashing payloads, which calculates the root and items
/// for Merkleised payloads, either messages or a new verifier set.
fn hash_payload_internal(
    payload: Payload,
    domain_separator: [u8; 32],
    signing_verifier_set_merkle_root: [u8; 32],
) -> Result<([u8; 32], MerkleisedPayload), EncodingError> {
    let (payload_merkle_root, payload_items) = match payload {
        Payload::Messages(messages) => {
            let leaves = types::messages::merkle_tree_leaves(
                messages,
                domain_separator,
                signing_verifier_set_merkle_root,
            )?
            .collect::<Vec<_>>();
            let messages_merkle_tree = merkle_tree::<NativeHasher, MessageLeaf>(leaves.iter());
            let messages_merkle_root = messages_merkle_tree
                .root()
                .ok_or(EncodingError::CannotMerkeliseEmptyMessageSet)?;
            let messages = leaves
                .into_iter()
                .map(|leaf| {
                    let proof = messages_merkle_tree.proof(&[leaf.position.into()]);
                    MerkleisedMessage {
                        leaf,
                        proof: proof.to_bytes(),
                    }
                })
                .collect::<Vec<_>>();
            (
                messages_merkle_root,
                MerkleisedPayload::NewMessages { messages },
            )
        }
        Payload::NewVerifierSet(verifier_set) => {
            let new_verifier_set_merkle_root =
                verifier_set_hash::<NativeHasher>(&verifier_set, &domain_separator)?;
            let payload = MerkleisedPayload::VerifierSetRotation {
                new_verifier_set_merkle_root,
            };
            let payload_hash_to_sign = types::verifier_set::construct_payload_hash::<NativeHasher>(
                new_verifier_set_merkle_root,
                signing_verifier_set_merkle_root,
            );

            (payload_hash_to_sign, payload)
        }
    };
    Ok((payload_merkle_root, payload_items))
}

pub(crate) fn merkle_tree<'a, T: rs_merkle::Hasher, K: udigest::Digestable + 'a>(
    leaves: impl Iterator<Item = &'a K>,
) -> MerkleTree<T> {
    let leaves = leaves
        .map(|item| {
            let mut buffer = VecBuf(vec![]);
            item.unambiguously_encode(EncodeValue::new(&mut buffer));
            T::hash(&buffer.0)
        })
        .collect::<Vec<_>>();
    MerkleTree::<T>::from_leaves(&leaves)
}

/// Trait for hashing leaves within a Merkle tree, implemented by types that can
/// be digested.
pub trait LeafHash: udigest::Digestable {
    /// Returns a hashed representation of the implementing type.
    fn hash<T: rs_merkle::Hasher>(&self) -> T::Hash {
        let mut buffer = VecBuf(vec![]);
        self.unambiguously_encode(EncodeValue::new(&mut buffer));
        T::hash(&buffer.0)
    }
}
