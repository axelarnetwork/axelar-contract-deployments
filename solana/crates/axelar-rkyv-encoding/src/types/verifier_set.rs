use std::collections::BTreeMap;
use std::error::Error;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use crate::hasher::Hasher;
use crate::types::{ArchivedPublicKey, ArchivedU256, PublicKey, U256};
use crate::visitor::{ArchivedVisitor, Visitor};

type Signers = BTreeMap<PublicKey, U256>;

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct VerifierSet {
    pub(crate) created_at: u64,
    pub(crate) signers: Signers,
    pub(crate) threshold: U256,
}

impl VerifierSet {
    pub fn new(created_at: u64, signers: Signers, threshold: U256) -> Self {
        Self {
            created_at,
            signers,
            threshold,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        Visitor::visit_verifier_set(&mut hasher, self);
        hasher.finalize()
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

    pub fn threshold(&self) -> &U256 {
        &self.threshold
    }

    pub fn created_at(&self) -> u64 {
        self.created_at
    }
}

impl ArchivedVerifierSet {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        ArchivedVisitor::visit_verifier_set(&mut hasher, self);
        hasher.finalize()
    }

    pub fn signers(&self) -> impl Iterator<Item = (&ArchivedPublicKey, &ArchivedU256)> {
        self.signers.iter()
    }

    pub fn size(&self) -> usize {
        self.signers.len()
    }

    pub fn threshold(&self) -> &ArchivedU256 {
        &self.threshold
    }

    pub fn is_empty(&self) -> bool {
        self.signers.is_empty()
    }

    /// Returns `None` on arithmetic overflows
    pub fn sufficient_weight(&self) -> Option<bool> {
        use bnum::types::U256 as BnumU256;
        self.signers
            .values()
            .try_fold(BnumU256::ZERO, |acc, weight| acc.checked_add(weight.into()))
            .map(|total_weight| (total_weight) >= ((&self.threshold).into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::random_valid_verifier_set;

    #[test]
    fn archived_and_unarchived_values_have_the_same_hash() {
        let verifier_set = random_valid_verifier_set();

        let serialized = rkyv::to_bytes::<_, 1024>(&verifier_set).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<VerifierSet>(&serialized) };

        assert_eq!(archived.hash(), verifier_set.hash());
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
        verifier_set.threshold = bnum::types::U256::ONE
            .checked_add(verifier_set.threshold.into())
            .unwrap()
            .into();

        let serialized = rkyv::to_bytes::<_, 1024>(&verifier_set).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<VerifierSet>(&serialized) };

        assert!(!archived.sufficient_weight().expect("no overflow"))
    }
}
