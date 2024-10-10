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
use std::ops::{Deref, DerefMut};

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

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }
}
impl Deref for HasheableMessageVec {
    type Target = [Message];

    fn deref(&self) -> &Self::Target {
        &self.inner_vec
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
}

impl Deref for ArchivedHasheableMessageVec {
    type Target = ArchivedVec<ArchivedMessage>;

    fn deref(&self) -> &Self::Target {
        &self.inner_vec
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

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }
}

impl Deref for HasheableSignersBTreeMap {
    type Target = BTreeMap<PublicKey, U128>;

    fn deref(&self) -> &Self::Target {
        &self.inner_map
    }
}

impl ArchivedHasheableSignersBTreeMap {
    pub fn new(inner_map: ArchivedBTreeMap<ArchivedPublicKey, ArchivedU128>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_map.len()),
            inner_map,
        }
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }
}

impl Deref for ArchivedHasheableSignersBTreeMap {
    type Target = ArchivedBTreeMap<ArchivedPublicKey, ArchivedU128>;

    fn deref(&self) -> &Self::Target {
        &self.inner_map
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

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }
}

impl Deref for HasheableSignersWithSignaturesBTreeMap {
    type Target = BTreeMap<PublicKey, WeightedSigner>;

    fn deref(&self) -> &Self::Target {
        &self.inner_map
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl DerefMut for HasheableSignersWithSignaturesBTreeMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner_map
    }
}

impl ArchivedHasheableSignersWithSignaturesBTreeMap {
    pub fn new(inner_map: ArchivedBTreeMap<ArchivedPublicKey, ArchivedWeightedSigner>) -> Self {
        Self {
            len_le_bytes: usize_to_le_len_bytes(inner_map.len()),
            inner_map,
        }
    }

    pub fn len_le_bytes(&self) -> &[u8; 4] {
        &self.len_le_bytes
    }
}

impl Deref for ArchivedHasheableSignersWithSignaturesBTreeMap {
    type Target = ArchivedBTreeMap<ArchivedPublicKey, ArchivedWeightedSigner>;

    fn deref(&self) -> &Self::Target {
        &self.inner_map
    }
}
