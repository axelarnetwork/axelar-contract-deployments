use rkyv::{Archive, Deserialize, Serialize};

use crate::types::{Message, VerifierSet};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub enum Payload {
    Messages(Vec<Message>),
    VerifierSet(VerifierSet),
}

impl Payload {
    pub fn new_messages(messages: Vec<Message>) -> Self {
        Self::Messages(messages)
    }

    pub fn new_verifier_set(verifier_set: VerifierSet) -> Self {
        Self::VerifierSet(verifier_set)
    }
}
