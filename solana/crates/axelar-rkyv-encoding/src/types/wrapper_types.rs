//! This module implements the wrapper types which holds precomputed values for
//! hashing.
//!
//! Some fields needs to be pre-computed, because we need to be able to only
//! borrow those fields to the underlying hashing infrastructure. Normally this
//! hashing syscalls take the raw pointers of this fields in order to access
//! them from the native platform code.
//!
//! See original issue for more information: https://github.com/eigerco/solana-axelar-internal/issues/361

use std::collections::BTreeMap;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::collections::ArchivedBTreeMap;
use rkyv::vec::ArchivedVec;
use rkyv::{Archive, Deserialize, Serialize};

use super::{
    ArchivedMessage, ArchivedPublicKey, ArchivedU128, ArchivedWeightedSigner, Message, PublicKey,
    WeightedSigner, U128,
};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct HasheableMessageVec {
    // adjusted to 32 bits to accommodate wasm32
    len_le_bytes: [u8; 4],
    inner_vec: Vec<Message>,
}

impl HasheableMessageVec {
    pub fn new(inner_vec: Vec<Message>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_vec.len()),
            inner_vec,
        }
    }

    pub fn as_slice(&self) -> &[Message] {
        self.inner_vec.as_slice()
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }

    #[allow(dead_code)]
    pub(crate) fn inner_vec(self) -> Vec<Message> {
        self.inner_vec
    }

    pub fn iter(&self) -> std::slice::Iter<Message> {
        self.inner_vec.iter()
    }
}

fn usize_to_le_len_bytes(data: usize) -> [u8; 4] {
    u32::try_from(data)
        .expect("usize too large to map it to u32")
        .to_le_bytes()
}

impl FromIterator<Message> for HasheableMessageVec {
    fn from_iter<T: IntoIterator<Item = Message>>(iter: T) -> Self {
        let inner_vec: Vec<Message> = iter.into_iter().collect();
        HasheableMessageVec::new(inner_vec)
    }
}

impl ArchivedHasheableMessageVec {
    pub fn new(inner_vec: ArchivedVec<ArchivedMessage>) -> Self {
        Self {
            len_le_bytes: u32::try_from(inner_vec.len())
                .expect("len too large")
                .to_le_bytes(),
            inner_vec,
        }
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }

    pub fn iter(&self) -> std::slice::Iter<'_, ArchivedMessage> {
        self.inner_vec.iter()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct HasheableSignersBTreeMap {
    // adjusted to 32 bits to accommodate wasm32
    len_le_bytes: [u8; 4],
    inner_map: BTreeMap<PublicKey, U128>,
}

impl HasheableSignersBTreeMap {
    pub fn new(inner_map: BTreeMap<PublicKey, U128>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_map.len()),
            inner_map,
        }
    }

    pub fn len(&self) -> usize {
        self.inner_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner_map.is_empty()
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }

    pub fn keys(&self) -> std::collections::btree_map::Keys<PublicKey, U128> {
        self.inner_map.keys()
    }

    pub fn values(&self) -> std::collections::btree_map::Values<PublicKey, U128> {
        self.inner_map.values()
    }

    pub(crate) fn inner_map(&self) -> &BTreeMap<PublicKey, U128> {
        &self.inner_map
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<PublicKey, U128> {
        self.inner_map.iter()
    }
}

impl ArchivedHasheableSignersBTreeMap {
    pub fn new(inner_map: ArchivedBTreeMap<ArchivedPublicKey, ArchivedU128>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_map.len()),
            inner_map,
        }
    }

    pub fn len(&self) -> usize {
        self.inner_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner_map.is_empty()
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }

    pub fn keys(&self) -> rkyv::collections::btree_map::Keys<ArchivedPublicKey, ArchivedU128> {
        self.inner_map.keys()
    }

    pub fn values(&self) -> rkyv::collections::btree_map::Values<ArchivedPublicKey, ArchivedU128> {
        self.inner_map.values()
    }

    #[allow(dead_code)]
    pub(crate) fn inner_map(&self) -> &ArchivedBTreeMap<ArchivedPublicKey, ArchivedU128> {
        &self.inner_map
    }

    pub fn iter(&self) -> rkyv::collections::btree_map::Iter<'_, ArchivedPublicKey, ArchivedU128> {
        self.inner_map.iter()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct HasheableSignersWithSignaturesBTreeMap {
    // adjusted to 32 bits to accommodate wasm32
    len_le_bytes: [u8; 4],
    inner_map: BTreeMap<PublicKey, WeightedSigner>,
}

impl HasheableSignersWithSignaturesBTreeMap {
    pub fn new(inner_map: BTreeMap<PublicKey, WeightedSigner>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_map.len()),
            inner_map,
        }
    }

    pub fn len(&self) -> usize {
        self.inner_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner_map.is_empty()
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }

    pub fn keys(&self) -> std::collections::btree_map::Keys<PublicKey, WeightedSigner> {
        self.inner_map.keys()
    }

    pub fn values(&self) -> std::collections::btree_map::Values<PublicKey, WeightedSigner> {
        self.inner_map.values()
    }

    #[allow(dead_code)]
    pub(crate) fn inner_map(&self) -> &BTreeMap<PublicKey, WeightedSigner> {
        &self.inner_map
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mut_inner_map(&mut self) -> &mut BTreeMap<PublicKey, WeightedSigner> {
        &mut self.inner_map
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<PublicKey, WeightedSigner> {
        self.inner_map.iter()
    }
}

impl ArchivedHasheableSignersWithSignaturesBTreeMap {
    pub fn new(inner_map: ArchivedBTreeMap<ArchivedPublicKey, ArchivedWeightedSigner>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_map.len()),
            inner_map,
        }
    }

    pub fn len(&self) -> usize {
        self.inner_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner_map.is_empty()
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }

    pub fn keys(
        &self,
    ) -> rkyv::collections::btree_map::Keys<ArchivedPublicKey, ArchivedWeightedSigner> {
        self.inner_map.keys()
    }

    pub fn values(
        &self,
    ) -> rkyv::collections::btree_map::Values<ArchivedPublicKey, ArchivedWeightedSigner> {
        self.inner_map.values()
    }

    pub(crate) fn inner_map(&self) -> &ArchivedBTreeMap<ArchivedPublicKey, ArchivedWeightedSigner> {
        &self.inner_map
    }

    pub fn iter(
        &self,
    ) -> rkyv::collections::btree_map::Iter<'_, ArchivedPublicKey, ArchivedWeightedSigner> {
        self.inner_map.iter()
    }
}
