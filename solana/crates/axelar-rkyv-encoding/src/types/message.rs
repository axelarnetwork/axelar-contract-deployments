use std::error::Error;

use rkyv::bytecheck::{self, CheckBytes, StructCheckError};
use rkyv::validation::validators::DefaultValidatorError;
use rkyv::{Archive, Deserialize, Serialize};

use crate::hasher::AxelarRkyv256Hasher;
use crate::visitor::{ArchivedVisitor, Visitor};

const COMMAND_ID_SEPARATOR: &str = "_";

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct CrossChainId {
    pub(crate) chain: String,
    pub(crate) id: String,
}

impl CrossChainId {
    pub fn new(chain: String, id: String) -> Self {
        Self { chain, id }
    }
    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        Visitor::visit_cc_id(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn chain(&self) -> &str {
        &self.chain
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn command_id<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        Visitor::visit_string(&mut hasher_impl, &self.chain);
        Visitor::visit_string(&mut hasher_impl, COMMAND_ID_SEPARATOR);
        Visitor::visit_string(&mut hasher_impl, &self.id);

        hasher_impl.result().into()
    }
}

impl ArchivedCrossChainId {
    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        ArchivedVisitor::visit_cc_id(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn chain(&self) -> &str {
        &self.chain
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn command_id<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        Visitor::visit_string(&mut hasher_impl, &self.chain);
        Visitor::visit_string(&mut hasher_impl, COMMAND_ID_SEPARATOR);
        Visitor::visit_string(&mut hasher_impl, &self.id);

        hasher_impl.result().into()
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct Message {
    pub(crate) cc_id: CrossChainId,
    pub(crate) source_address: String,
    pub(crate) destination_chain: String,
    pub(crate) destination_address: String,
    pub(crate) payload_hash: [u8; 32],
}

impl Message {
    pub fn new(
        cc_id: CrossChainId,
        source_address: String,
        destination_chain: String,
        destination_address: String,
        payload_hash: [u8; 32],
    ) -> Self {
        Self {
            cc_id,
            source_address,
            destination_chain,
            destination_address,
            payload_hash,
        }
    }

    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        Visitor::visit_message(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn cc_id(&self) -> &CrossChainId {
        &self.cc_id
    }

    pub fn destination_address(&self) -> &str {
        &self.destination_address
    }

    pub fn source_address(&self) -> &str {
        &self.source_address
    }

    pub fn payload_hash(&self) -> &[u8; 32] {
        &self.payload_hash
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
}

impl ArchivedMessage {
    pub fn hash<'a>(&'a self, mut hasher_impl: impl AxelarRkyv256Hasher<'a>) -> [u8; 32] {
        ArchivedVisitor::visit_message(&mut hasher_impl, self);
        hasher_impl.result().into()
    }

    pub fn cc_id(&self) -> &ArchivedCrossChainId {
        &self.cc_id
    }

    pub fn destination_address(&self) -> &str {
        &self.destination_address
    }

    pub fn source_address(&self) -> &str {
        &self.source_address
    }

    pub fn payload_hash(&self) -> &[u8; 32] {
        &self.payload_hash
    }

    pub fn from_archived_bytes(
        bytes: &[u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<StructCheckError, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<Message>(bytes)
    }
}

/// Metadata that belongs to a Gmp message. This is mainly used in
/// special cases in which the relayer is aware and does a special message
/// handling.
///
/// See ITS and Governance contracts as examples of usage.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct GmpMetadata {
    /// The cross-chain id
    pub cross_chain_id: CrossChainId,

    /// Address of the source contract
    pub source_address: String,

    /// Address of the destination contract
    pub destination_address: String,

    /// Id of the destination chain
    pub destination_chain: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{random_message, test_hasher_impl};

    #[test]
    fn unarchived_roundtrip() {
        let message = random_message();

        let bytes = message.to_bytes().unwrap();
        let deserialized = Message::from_bytes(&bytes).unwrap();

        assert_eq!(message, deserialized);
        assert_eq!(
            message.hash(test_hasher_impl()),
            deserialized.hash(test_hasher_impl())
        );
    }

    #[test]
    fn consistent_hash_across_archival() {
        let message = random_message();

        let bytes = message.to_bytes().unwrap();
        let archived = ArchivedMessage::from_archived_bytes(&bytes).unwrap();

        assert_eq!(
            message.hash(test_hasher_impl()),
            archived.hash(test_hasher_impl())
        );
    }
}
