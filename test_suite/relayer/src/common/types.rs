use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct CcId {
    pub chain: String,
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub cc_id: CcId,
    pub source_address: String,
    pub destination_chain: String,
    pub destination_address: String,
    pub payload_hash: String,
}
