use std::collections::BTreeMap;
use std::error::Error;
use std::marker::PhantomData;
use std::ops::Deref;

use rkyv::bytecheck::{self, CheckBytes, StructCheckError};
use rkyv::validation::validators::DefaultValidatorError;
use rkyv::{Archive, Deserialize, Serialize};

use super::HasheableSignersBTreeMap;
use crate::hasher::generic::Keccak256Hasher;
use crate::hasher::merkle_trait::Merkle;
use crate::hasher::merkle_tree::{NativeHasher, SolanaSyscallHasher};
use crate::hasher::solana::SolanaKeccak256Hasher;
use crate::hasher::AxelarRkyv256Hasher;
use crate::types::{ArchivedPublicKey, ArchivedU128, PublicKey, U128};
use crate::visitor::{ArchivedVisitor, Visitor};

type Signers = BTreeMap<PublicKey, U128>;

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct VerifierSet {
    pub(crate) created_at: u64,
    created_at_le_bytes: [u8; 8],
    pub(crate) signers: HasheableSignersBTreeMap,
    pub(crate) quorum: U128,
    pub(crate) domain_separator: [u8; 32],
}

impl VerifierSet {
    pub fn new(
        created_at: u64,
        signers: Signers,
        quorum: U128,
        domain_separator: [u8; 32],
    ) -> Self {
        Self {
            created_at,
            created_at_le_bytes: created_at.to_le_bytes(),
            signers: HasheableSignersBTreeMap::new(signers),
            quorum,
            domain_separator,
        }
    }

    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        Visitor::visit_verifier_set(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        rkyv::to_bytes::<_, 0>(self)
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
            .map(|bytes| bytes.to_vec())
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    pub fn signers(&self) -> &Signers {
        &self.signers
    }

    pub fn quorum(&self) -> &U128 {
        &self.quorum
    }

    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    pub fn created_at_le_bytes(&self) -> &[u8; 8] {
        &self.created_at_le_bytes
    }

    pub fn element_iterator(&self) -> impl Iterator<Item = VerifierSetElement> + '_ {
        self.signers()
            .iter()
            .enumerate()
            .map(
                |(position, (signer_pubkey, signer_weight))| VerifierSetElement {
                    created_at: self.created_at,
                    quorum: self.quorum.into(),
                    domain_separator: self.domain_separator,
                    signer_pubkey: *signer_pubkey,
                    signer_weight: (*signer_weight).into(),
                    position: position as u16,
                    set_size: self.signers.len() as u16,
                },
            )
    }
}

impl ArchivedVerifierSet {
    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        ArchivedVisitor::visit_verifier_set(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn signers(&self) -> impl Iterator<Item = (&ArchivedPublicKey, &ArchivedU128)> {
        self.signers.iter()
    }

    pub fn size(&self) -> usize {
        self.signers.len()
    }

    pub fn quorum(&self) -> &ArchivedU128 {
        &self.quorum
    }

    pub fn is_empty(&self) -> bool {
        self.signers.is_empty()
    }

    /// Returns `None` on arithmetic overflows
    pub fn sufficient_weight(&self) -> Option<bool> {
        use bnum::types::U128 as BnumU128;
        self.signers
            .values()
            .try_fold(BnumU128::ZERO, |acc, weight| acc.checked_add(weight.into()))
            .map(|total_weight| (total_weight) >= ((&self.quorum).into()))
    }

    pub fn from_archived_bytes(
        bytes: &[u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<StructCheckError, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<VerifierSet>(bytes)
    }

    pub fn created_at_le_bytes(&self) -> &[u8; 8] {
        &self.created_at_le_bytes
    }
}

/// A `VerifierSet` element.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct VerifierSetElement {
    pub created_at: u64,
    pub quorum: u128,
    pub signer_pubkey: PublicKey,
    pub signer_weight: u128,
    pub domain_separator: [u8; 32],
    pub position: u16,
    pub set_size: u16,
}

/// Wraps a [`VerifierSetElement`], is generic over the hashing context.
///
/// This type is the leaf node of a [`VerifierSet`]'s Merkle tree.
#[derive(Clone, Copy)]
pub struct VerifierSetLeafNode<T: rs_merkle::Hasher<Hash = [u8; 32]>> {
    element: VerifierSetElement,
    hasher: PhantomData<T>,
}

impl<T: rs_merkle::Hasher<Hash = [u8; 32]>> Deref for VerifierSetLeafNode<T> {
    type Target = VerifierSetElement;

    fn deref(&self) -> &Self::Target {
        &self.element
    }
}

impl<'a, T> VerifierSetLeafNode<T>
where
    T: rs_merkle::Hasher<Hash = [u8; 32]>,
{
    /// Converts this leaf node into bytes that will become the leaf nodes of a
    /// [`VerifierSet`]'s Merkle tree.
    #[inline]
    pub fn leaf_hash<H>(&'a self) -> [u8; 32]
    where
        H: AxelarRkyv256Hasher<'a>,
    {
        let mut hasher = H::default();
        hasher.hash(&[0]); // Leaf node discriminator
        hasher.hash(b"verifier-set");
        hasher.hash(bytemuck::cast_ref::<_, [u8; 8]>(&self.element.created_at));
        hasher.hash(bytemuck::cast_ref::<_, [u8; 16]>(&self.element.quorum));
        hasher.hash(&self.element.domain_separator);
        Visitor::visit_public_key(&mut hasher, &self.element.signer_pubkey);
        hasher.hash(bytemuck::cast_ref::<_, [u8; 16]>(
            &self.element.signer_weight,
        ));
        hasher.hash(bytemuck::cast_ref::<_, [u8; 2]>(&self.element.position));
        hasher.result().into()
    }
}

impl<T> From<VerifierSetElement> for VerifierSetLeafNode<T>
where
    T: rs_merkle::Hasher<Hash = [u8; 32]>,
{
    fn from(element: VerifierSetElement) -> Self {
        Self {
            element,
            hasher: PhantomData,
        }
    }
}

impl From<VerifierSetLeafNode<SolanaSyscallHasher>> for [u8; 32] {
    fn from(leaf_node: VerifierSetLeafNode<SolanaSyscallHasher>) -> Self {
        leaf_node.leaf_hash::<SolanaKeccak256Hasher>()
    }
}
impl From<VerifierSetLeafNode<NativeHasher>> for [u8; 32] {
    fn from(leaf_node: VerifierSetLeafNode<NativeHasher>) -> Self {
        leaf_node.leaf_hash::<Keccak256Hasher>()
    }
}

impl<H> Merkle<H> for VerifierSet
where
    H: rs_merkle::Hasher<Hash = [u8; 32]>,
    VerifierSetLeafNode<H>: Into<[u8; 32]>,
{
    type LeafNode = VerifierSetLeafNode<H>;

    fn merkle_leaves(&self) -> impl Iterator<Item = VerifierSetLeafNode<H>> {
        self.element_iterator().map(|element| VerifierSetLeafNode {
            element,
            hasher: PhantomData,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hasher::merkle_trait::tests::assert_merkle_inclusion_proof;
    use crate::test_fixtures::{random_valid_verifier_set, test_hasher_impl};

    #[test]
    fn archived_and_unarchived_values_have_the_same_hash() {
        let verifier_set = random_valid_verifier_set();

        let serialized = rkyv::to_bytes::<_, 1024>(&verifier_set).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<VerifierSet>(&serialized) };

        assert_eq!(
            archived.hash(test_hasher_impl()),
            verifier_set.hash(test_hasher_impl())
        );
    }

    #[test]
    fn unarchived_roundtrip() {
        let verifier_set = random_valid_verifier_set();

        let bytes = verifier_set.to_bytes().unwrap();
        let deserialized = VerifierSet::from_bytes(&bytes).unwrap();

        assert_eq!(verifier_set, deserialized);
    }

    #[test]
    fn sufficient_weight() {
        let verifier_set = random_valid_verifier_set();

        let serialized = rkyv::to_bytes::<_, 1024>(&verifier_set).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<VerifierSet>(&serialized) };

        assert!(archived.sufficient_weight().expect("no overflow"))
    }

    #[test]
    fn insufficient_weight() {
        let mut verifier_set = random_valid_verifier_set();

        // Fixture VerifierSet threshold values are always equal to the sum of signer
        // weights. Let's bump that.
        verifier_set.quorum = bnum::types::U128::ONE
            .checked_add(verifier_set.quorum.into())
            .unwrap()
            .into();

        let serialized = rkyv::to_bytes::<_, 1024>(&verifier_set).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<VerifierSet>(&serialized) };

        assert!(!archived.sufficient_weight().expect("no overflow"))
    }

    #[test]
    fn test_verifier_set_merkle_inclusion_proof() {
        let verifier_set = random_valid_verifier_set();

        assert_eq!(
            assert_merkle_inclusion_proof::<SolanaSyscallHasher, _>(&verifier_set),
            assert_merkle_inclusion_proof::<NativeHasher, _>(&verifier_set),
            "different hasher implementations should produce the same merkle root"
        )
    }
}
