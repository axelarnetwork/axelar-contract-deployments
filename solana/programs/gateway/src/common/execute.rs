use super::*;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Input {
    pub proof: Vec<u8>,
    pub data: Vec<u8>,
}

impl Input {
    fn new(proof: Vec<u8>, data: Vec<u8>) -> Self {
        Input { proof, data }
    }
}

impl Input {
    pub fn encode(self) -> Vec<u8> {
        self.try_to_vec().unwrap()
    }

    pub fn decode(encoded: Vec<u8>) -> Self {
        Input::try_from_slice(&encoded).unwrap()
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Data {
    pub chain_id: [u8; 256],
    pub command_ids: Vec<[u8; 32]>,
    pub commands: Vec<String>,
    pub params: Vec<u8>,
}

impl Data {
    fn new(
        chain_id: [u8; 256],
        command_ids: Vec<[u8; 32]>,
        commands: Vec<String>,
        params: Vec<u8>,
    ) -> Self {
        Data {
            chain_id,
            command_ids,
            commands,
            params,
        }
    }
}

impl Data {
    pub fn encode(self) -> Vec<u8> {
        self.try_to_vec().unwrap()
    }

    pub fn decode(encoded: Vec<u8>) -> Self {
        Data::try_from_slice(&encoded).unwrap()
    }
}
