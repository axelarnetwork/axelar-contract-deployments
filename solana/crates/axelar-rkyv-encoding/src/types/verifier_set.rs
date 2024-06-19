use std::collections::BTreeMap;

use rkyv::{Archive, Deserialize, Serialize};

use crate::types::{Signer, U256};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct VerifierSet {
    pub(crate) created_at: u64,
    pub(crate) signers: BTreeMap<String, Signer>,
    pub(crate) threshold: U256,
}

impl VerifierSet {
    pub fn new(created_at: u64, signers: BTreeMap<String, Signer>, threshold: U256) -> Self {
        Self {
            created_at,
            signers,
            threshold,
        }
    }
}
