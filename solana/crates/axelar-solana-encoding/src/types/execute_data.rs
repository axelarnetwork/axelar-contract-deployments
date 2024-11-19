//! # `ExecuteData` Module
//!
//! This module defines the `ExecuteData` struct and its related components,
//! which are essential for encoding and verifying data integrity on Solana.

use super::messages::MessageLeaf;
use super::pubkey::Signature;
use super::verifier_set::VerifierSetLeaf;

/// Represents the complete set of execution data required for verification and
/// processing.
///
/// `ExecuteData` includes Merkle roots for the signing verifier set and the
/// payload, as well as detailed information about each verifier's signature and
/// the structure of the payload.
#[derive(Debug, Eq, PartialEq, Clone, borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct ExecuteData {
    /// The Merkle root of the signing verifier set.
    pub signing_verifier_set_merkle_root: [u8; 32],

    /// A list of information about each verifier in the signing set, including
    /// their signatures and Merkle proofs.
    pub signing_verifier_set_leaves: Vec<SigningVerifierSetInfo>,

    /// The Merkle root of the payload data.
    pub payload_merkle_root: [u8; 32],

    /// The payload items, which can either be new messages or a verifier set
    /// rotation, each accompanied by their respective Merkle proofs.
    pub payload_items: MerkleisedPayload,
}

/// Contains information about a single verifier within the signing verifier
/// set.
///
/// This struct holds the verifier's signature, their corresponding leaf in the
/// verifier set Merkle tree, and the Merkle proof needed to verify their
/// inclusion in the set.
#[derive(Debug, Eq, PartialEq, Clone, borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct SigningVerifierSetInfo {
    /// The signature provided by the verifier.
    pub signature: Signature,

    /// The leaf node representing the verifier in the Merkle tree.
    pub leaf: VerifierSetLeaf,

    /// The Merkle proof demonstrating the verifier's inclusion in the signing
    /// verifier set.
    pub merkle_proof: Vec<u8>,
}

/// Represents the payload data in a Merkle tree structure.
///
/// `MerkleisedPayload` can either be a rotation of the verifier set or a
/// collection of new messages, each accompanied by their respective Merkle
/// proofs.
#[derive(Debug, Eq, PartialEq, Clone, borsh::BorshDeserialize, borsh::BorshSerialize)]
pub enum MerkleisedPayload {
    /// Indicates a rotation of the verifier set, providing the new Merkle root
    /// of the verifier set.
    VerifierSetRotation {
        /// The Merkle root of the new verifier set after rotation.
        new_verifier_set_merkle_root: [u8; 32],
    },

    /// Contains a list of new messages, each with its corresponding Merkle
    /// proof.
    NewMessages {
        /// A vector of `MerkleisedMessage` instances, each representing a
        /// message and its proof.
        messages: Vec<MerkleisedMessage>,
    },
}

/// Represents a single message within the payload, along with its Merkle proof.
///
/// Each `MerkleisedMessage` includes the message content encapsulated in a
/// `MessageLeaf` and a proof that verifies the message's inclusion in the
/// Merkle tree.
#[derive(Debug, Eq, PartialEq, Clone, borsh::BorshDeserialize, borsh::BorshSerialize)]
pub struct MerkleisedMessage {
    /// The leaf node representing the message in the Merkle tree.
    pub leaf: MessageLeaf,

    /// The Merkle proof demonstrating the message's inclusion in the payload's
    /// Merkle tree.
    pub proof: Vec<u8>,
}
