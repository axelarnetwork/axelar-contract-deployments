use gateway::instructions::ContractCallEvent;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CcId {
    pub chain: String,
    pub id: String,
}

impl CcId {
    pub fn from_chain_signature_and_index(
        chain: String,
        signature: Signature,
        index: usize,
    ) -> Self {
        CcId {
            chain: chain,
            id: format!("{}:{}", signature, index),
        }
    }

    pub fn to_signature_and_index(self) -> (String, usize) {
        let result: Vec<&str> = self.id.split(':').collect();
        let signature = result[0].to_string();
        let index = result[1].parse::<usize>().unwrap();
        (signature, index)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Message {
    pub cc_id: CcId,
    pub source_address: String,
    pub destination_chain: String,
    pub destination_address: String,
    pub payload_hash: String,
}

impl Message {
    pub fn prepare_message_for_axelar_side(cc_id: CcId, event_body: &ContractCallEvent) -> Self {
        let payload_hash_hex: String = event_body
            .payload_hash
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect();

        Self {
            cc_id,
            source_address: event_body.sender.to_string(),
            destination_chain: event_body.destination_chain.clone(),
            destination_address: event_body.destination_contract_address.clone(),
            payload_hash: payload_hash_hex,
        }
    }
}
