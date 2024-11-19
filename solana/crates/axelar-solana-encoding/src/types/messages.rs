//! # Messages Module
//!
//! This module defines the core structures and functions related to handling
//! messages within Axelar cross-chain system

use crate::error::EncodingError;
use crate::LeafHash;

/// Represents a collection of `Message` instances.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Messages(pub Vec<Message>);

/// Identifies a specific blockchain and its unique identifier within that
/// chain.
#[derive(
    Clone, PartialEq, Eq, Debug, udigest::Digestable, borsh::BorshDeserialize, borsh::BorshSerialize,
)]
pub struct CrossChainId {
    /// The name or identifier of the source blockchain.
    pub chain: String,

    /// A unique identifier within the specified blockchain.
    pub id: String,
}

/// Represents a message intended for cross-chain communication.
#[derive(
    Clone, PartialEq, Eq, Debug, udigest::Digestable, borsh::BorshDeserialize, borsh::BorshSerialize,
)]
pub struct Message {
    /// The cross-chain identifier of the message
    pub cc_id: CrossChainId,

    /// The source address from which the message originates.
    pub source_address: String,

    /// The destination blockchain where the message is intended to be sent.
    pub destination_chain: String,

    /// The destination address on the target blockchain.
    pub destination_address: String,

    /// A 32-byte hash of the message payload, ensuring data integrity.
    pub payload_hash: [u8; 32],
}

/// Generates an iterator of `MessageLeaf` instances from a collection of
/// messages.
pub(crate) fn merkle_tree_leaves(
    messages: Messages,
    domain_separator: [u8; 32],
    signing_verifier_set: [u8; 32],
) -> Result<impl Iterator<Item = MessageLeaf>, EncodingError> {
    let set_size = messages
        .0
        .len()
        .try_into()
        .map_err(|_err| EncodingError::SetSizeTooLarge)?;
    let iterator = messages
        .0
        .into_iter()
        .enumerate()
        .map(move |(position, message)| MessageLeaf {
            domain_separator,
            position: position
                .try_into()
                .expect("position guaranteed to equal set size"),
            set_size,
            message,
            signing_verifier_set,
        });
    Ok(iterator)
}

/// Represents a leaf node in a Merkle tree for a `Message`.
///
/// The `MessageLeaf` struct includes the message itself along with metadata
/// required for Merkle tree operations, such as its position within the tree,
/// the total size of the set, a domain separator, and the Merkle root of the
/// signing verifier set.
#[derive(
    Clone, PartialEq, Eq, Debug, udigest::Digestable, borsh::BorshDeserialize, borsh::BorshSerialize,
)]
pub struct MessageLeaf {
    /// The message contained within this leaf node.
    pub message: Message,

    /// The position of this leaf within the Merkle tree.
    pub position: u16,

    /// The total number of leaves in the Merkle tree.
    pub set_size: u16,

    /// A domain separator used to ensure the uniqueness of hashes across
    /// different contexts.
    pub domain_separator: [u8; 32],

    /// The Merkle root of the signing verifier set, used for verifying
    /// signatures.
    pub signing_verifier_set: [u8; 32],
}

impl LeafHash for MessageLeaf {}
