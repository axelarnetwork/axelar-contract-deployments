use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Params {
    pub source_chain: String,
    pub source_address: String,
    pub contract_address: String,
    pub payload_hash: [u8; 32],
    pub source_tx_hash: [u8; 32],
    pub source_event_index: [u8; 256],
}

impl Params {
    pub fn decode(encoded: Vec<u8>) -> Self {
        Params::try_from_slice(&encoded).unwrap()
    }
}
