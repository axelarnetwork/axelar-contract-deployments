use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct CrossChainId {
    pub(crate) chain: String,
    pub(crate) id: String,
}

impl CrossChainId {
    pub fn new(chain: String, id: String) -> Self {
        Self { chain, id }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
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
}
