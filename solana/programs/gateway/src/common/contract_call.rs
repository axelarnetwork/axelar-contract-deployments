use super::*;

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
    fn new(
        source_chain: String,
        source_address: String,
        contract_address: String,
        payload_hash: [u8; 32],
        source_tx_hash: [u8; 32],
        source_event_index: [u8; 256], // TODO: check if event index is needed
    ) -> Self {
        Params {
            source_chain,
            source_address,
            contract_address,
            payload_hash,
            source_tx_hash,
            source_event_index,
        }
    }
}

impl Params {
    pub fn encode(self) -> Vec<u8> {
        self.try_to_vec().unwrap()
    }

    pub fn decode(encoded: Vec<u8>) -> Self {
        Params::try_from_slice(&encoded).unwrap()
    }
}
