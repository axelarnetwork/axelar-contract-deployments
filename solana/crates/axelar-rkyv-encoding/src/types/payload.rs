use std::iter;
use std::marker::PhantomData;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use super::{HasheableMessageVec, Message};
use crate::hasher::generic::Keccak256Hasher;
use crate::hasher::merkle_trait::Merkle;
use crate::hasher::merkle_tree::NativeHasher;
use crate::hasher::AxelarRkyv256Hasher;
#[cfg(any(test, feature = "test-fixtures", feature = "solana"))]
use crate::hasher::{merkle_tree::SolanaSyscallHasher, solana::SolanaKeccak256Hasher};
use crate::types::VerifierSet;

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Payload {
    Messages(HasheableMessageVec),
    VerifierSet(VerifierSet),
}

impl Payload {
    /// Creates a new `Payload` instance containing a vector of messages.
    pub fn new_messages(messages: Vec<Message>) -> Self {
        Self::Messages(HasheableMessageVec::new(messages))
    }

    /// Creates a new `Payload` instance containing a verifier set.
    pub fn new_verifier_set(verifier_set: VerifierSet) -> Self {
        Self::VerifierSet(verifier_set)
    }

    /// Returns the number of elements contained within the payload.
    pub fn element_count(&self) -> usize {
        match self {
            Payload::Messages(messages) => messages.len(),
            Payload::VerifierSet(_) => 1,
        }
    }

    /// Iterates over [`Payload`] and yields [`PayloadElement`] values.
    pub fn element_iterator(&self) -> impl Iterator<Item = PayloadElement> + '_ {
        let num_messages = self.element_count() as u16;
        let mut position = 0u16;
        iter::from_fn(move || {
            if position == num_messages {
                return None;
            }
            let element = match self {
                Payload::Messages(messages) => PayloadElement::Message(MessageElement {
                    message: (messages[position as usize]).clone(),
                    position,
                    num_messages,
                }),
                Payload::VerifierSet(verifier_set) => {
                    PayloadElement::VerifierSet(verifier_set.clone())
                }
            };
            position += 1;
            Some(element)
        })
    }
}

impl TryFrom<Payload> for HasheableMessageVec {
    type Error = ();
    fn try_from(value: Payload) -> Result<Self, Self::Error> {
        match value {
            Payload::Messages(messages) => Ok(messages),
            _ => Err(()),
        }
    }
}

impl TryFrom<Payload> for VerifierSet {
    type Error = ();
    fn try_from(value: Payload) -> Result<Self, Self::Error> {
        match value {
            Payload::VerifierSet(verifier_set) => Ok(verifier_set),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageElement {
    pub message: Message,
    pub position: u16,
    pub num_messages: u16,
}

/// A [`Payload`] element.
#[derive(Debug, Clone)]
pub enum PayloadElement {
    Message(MessageElement),
    VerifierSet(VerifierSet),
}

/// Wraps a [`PayloadElement`], is generic over the hashing context.
///
/// This type is the leaf node of a [`Payload`]'s Merkle tree.
#[derive(Debug, Clone)]
pub struct PayloadLeafNode<T> {
    pub element: PayloadElement,
    pub hasher: PhantomData<T>,
}

impl<'a, T> PayloadLeafNode<T>
where
    VerifierSet: Merkle<T>,
    T: rs_merkle::Hasher<Hash = [u8; 32]>,
{
    /// Converts this leaf node into bytes that will become the leaf nodes of a
    /// [`Payload`]'s Merkle tree.
    #[inline]
    pub fn leaf_hash<H>(&'a self) -> [u8; 32]
    where
        H: AxelarRkyv256Hasher<'a>,
    {
        match &self.element {
            PayloadElement::Message(MessageElement {
                message,
                position,
                num_messages,
            }) => {
                let mut hasher = H::default();
                hasher.hash(&[0]); // Leaf node discriminator
                hasher.hash(b"message");
                hasher.hash(bytemuck::cast_ref::<_, [u8; 2]>(position));
                hasher.hash(bytemuck::cast_ref::<_, [u8; 2]>(num_messages));
                message.hash(hasher)
            }
            PayloadElement::VerifierSet(verifier_set) => {
                // When the Payload contains a verifier set, we use the Merkle root for that
                // verifier set hash directly.
                let verifier_set_merkle_root =
                    <VerifierSet as Merkle<T>>::calculate_merkle_root(verifier_set)
                        .expect("Can't use an empty verifier set");
                let payload_element_leaf_hash =
                    H::hash_instant(&[VerifierSet::HASH_PREFIX, &verifier_set_merkle_root]);
                payload_element_leaf_hash.0
            }
        }
    }
}

#[cfg(any(test, feature = "test-fixtures", feature = "solana"))]
impl From<PayloadLeafNode<SolanaSyscallHasher>> for [u8; 32] {
    fn from(payload_leaf_node: PayloadLeafNode<SolanaSyscallHasher>) -> Self {
        payload_leaf_node.leaf_hash::<SolanaKeccak256Hasher>()
    }
}

impl From<PayloadLeafNode<NativeHasher>> for [u8; 32] {
    fn from(payload_leaf_node: PayloadLeafNode<NativeHasher>) -> Self {
        payload_leaf_node.leaf_hash::<Keccak256Hasher>()
    }
}

impl<H> Merkle<H> for Payload
where
    H: rs_merkle::Hasher<Hash = [u8; 32]>,
    PayloadLeafNode<H>: Into<[u8; 32]>,
{
    type LeafNode = PayloadLeafNode<H>;

    fn merkle_leaves(&self) -> impl Iterator<Item = PayloadLeafNode<H>> {
        self.element_iterator().map(|element| PayloadLeafNode {
            element,
            hasher: PhantomData,
        })
    }
}
