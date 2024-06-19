use rkyv::{Archive, Deserialize, Serialize};

use crate::types::{PublicKey, U256};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct Signer {
    pub(crate) address: String,
    pub(crate) public_key: PublicKey,
    pub(crate) weight: U256,
}

impl Signer {
    pub fn new(address: String, public_key: PublicKey, weight: U256) -> Self {
        Self {
            address,
            public_key,
            weight,
        }
    }
}
