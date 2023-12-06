use serde::{Deserialize, Serialize};
use solana_sdk::signature::Signature;
use std::{fmt::Write, string::FromUtf8Error};

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
            chain,
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
    pub fn prepare_message_for_axelar_side(cc_id: CcId, event_body: &[Vec<u8>]) -> Self {
        let payload_hash_hex: String = event_body[3].iter().fold(String::new(), |mut output, b| {
            let _ = write!(output, "{:02X}", b);
            output
        });

        let destination_chain_str = vec_to_string(event_body[1].clone()).unwrap(); // TODO:
        let destination_contract_addr = vec_to_string(event_body[2].clone()).unwrap(); // TODO:

        Self {
            cc_id,
            source_address: String::from_utf8(event_body[0].clone()).unwrap(),
            destination_chain: destination_chain_str,
            destination_address: destination_contract_addr,
            payload_hash: payload_hash_hex,
        }
    }
}

/// Convert Vec[u8] to [String].
fn vec_to_string(body: Vec<u8>) -> Result<String, FromUtf8Error> {
    String::from_utf8(body)
}
