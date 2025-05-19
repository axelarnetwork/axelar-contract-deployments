//! # Verifier Set Module
//!
//! This module defines the structures and functions related to managing and
//! hashing verifier sets within Solana. It includes the `VerifierSet` struct,
//! which represents a set of verifiers with associated weights, and the
//! `VerifierSetLeaf` struct, which serves as a leaf node in a Merkle tree for
//! verifier sets. The module also provides functions for constructing payload
//! hashes and generating Merkle tree leaves from verifier sets.

use std::collections::BTreeMap;

use super::pubkey::PublicKey;
use crate::error::EncodingError;
use crate::hasher::HashvSupport;
use crate::LeafHash;

/// Represents a set of verifiers, each with an associated weight, and a quorum
/// value.
///
/// The `VerifierSet` struct encapsulates a collection of verifiers identified
/// by their public keys, each assigned a specific weight. Additionally, it
/// includes a quorum value that may be used to determine consensus requirements
/// within the set.
#[derive(Debug, Eq, PartialEq, Clone, borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct VerifierSet {
    /// A nonce value that can be used to track changes or updates to the
    /// verifier set.
    pub nonce: u64,

    /// A map of public keys to their corresponding weights. Each entry
    /// represents a verifier and the weight assigned to their contribution
    /// or authority.
    pub signers: BTreeMap<PublicKey, u128>,

    /// The quorum value required for consensus or decision-making within the
    /// verifier set. This value typically represents the minimum total
    /// weight needed to approve an action.
    pub quorum: u128,
}

/// Constructs a hash for a payload involving a new verifier set and the current
/// signing verifier set.
///
/// The `construct_payload_hash` function creates a unique hash by combining a
/// prefix with the Merkle roots of the new verifier set and the current signing
/// verifier set. This hash can be used to authenticate and verify the integrity
/// of payloads related to verifier set rotations.
#[must_use]
pub fn construct_payload_hash<T: HashvSupport>(
    new_verifier_set_merkle_root: [u8; 32],
    signing_verifier_set_merkle_root: [u8; 32],
) -> [u8; 32] {
    const HASH_PREFIX: &[u8] = b"new verifier set";
    T::hashv(&[
        HASH_PREFIX,
        &new_verifier_set_merkle_root,
        &signing_verifier_set_merkle_root,
    ])
}

/// Generates the Merkle root hash for a given verifier set.
///
/// The `verifier_set_hash` function constructs a Merkle tree from the leaves
/// generated from the provided `VerifierSet` and returns the Merkle root. This
/// root can be used to verify the integrity and membership of verifiers within
/// the set.
///
/// # Errors
/// - if the verifier set has no entiers in it
pub fn verifier_set_hash<T: rs_merkle::Hasher>(
    verifier_set: &VerifierSet,
    domain_separator: &[u8; 32],
) -> Result<T::Hash, EncodingError> {
    let leaves = merkle_tree_leaves(verifier_set, domain_separator)?.collect::<Vec<_>>();
    let mt = crate::merkle_tree::<T, VerifierSetLeaf>(leaves.iter());

    mt.root()
        .ok_or(EncodingError::CannotMerkeliseEmptyVerifierSet)
}

pub(crate) fn merkle_tree_leaves<'a>(
    vs: &'a VerifierSet,
    domain_separator: &'a [u8; 32],
) -> Result<impl Iterator<Item = VerifierSetLeaf> + 'a, EncodingError> {
    let set_size = vs
        .signers
        .len()
        .try_into()
        .map_err(|_err| EncodingError::SetSizeTooLarge)?;
    let iterator =
        vs.signers
            .iter()
            .enumerate()
            .map(
                move |(position, (signer_pubkey, signer_weight))| VerifierSetLeaf {
                    nonce: vs.nonce,
                    quorum: vs.quorum,
                    domain_separator: *domain_separator,
                    signer_pubkey: *signer_pubkey,
                    signer_weight: *signer_weight,
                    position: position
                        .try_into()
                        .expect("position and set size ar guaranteed to be equal"),
                    set_size,
                },
            );
    Ok(iterator)
}

/// Represents a leaf node in a Merkle tree for a verifier set.
///
/// The `VerifierSetLeaf` struct encapsulates all necessary information about a
/// verifier within a verifier set, including their public key, weight, and
/// positional metadata. This struct is designed to be used as a leaf node in a
/// Merkle tree, facilitating efficient and secure verification of verifiers.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    Debug,
    udigest::Digestable,
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
)]
pub struct VerifierSetLeaf {
    /// The nonce value from the associated `VerifierSet`.
    pub nonce: u64,

    /// The quorum value from the associated `VerifierSet`.
    pub quorum: u128,

    /// The public key of the verifier.
    pub signer_pubkey: PublicKey,

    /// The weight assigned to the verifier, representing their voting power or
    /// authority.
    pub signer_weight: u128,

    /// The position of this leaf within the Merkle tree.
    pub position: u16,

    /// The total number of leaves in the Merkle tree, representing the size of
    /// the verifier set.
    pub set_size: u16,

    /// A domain separator used to ensure the uniqueness of hashes across
    /// different contexts.
    pub domain_separator: [u8; 32],
}

impl LeafHash for VerifierSetLeaf {}
