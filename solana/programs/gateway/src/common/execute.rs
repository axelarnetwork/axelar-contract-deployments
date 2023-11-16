use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Input {
    pub proof: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Data {
    pub chain_id: [u8; 256],
    pub command_ids: Vec<[u8; 32]>,
    pub commands: Vec<String>,
    pub params: Vec<u8>,
}
