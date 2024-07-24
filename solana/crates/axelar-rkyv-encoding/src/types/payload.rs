use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use super::{HasheableMessageVec, Message};
use crate::types::VerifierSet;

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Payload {
    Messages(HasheableMessageVec),
    VerifierSet(VerifierSet),
}

impl Payload {
    pub fn new_messages(messages: Vec<Message>) -> Self {
        Self::Messages(HasheableMessageVec::new(messages))
    }

    pub fn new_verifier_set(verifier_set: VerifierSet) -> Self {
        Self::VerifierSet(verifier_set)
    }
}
