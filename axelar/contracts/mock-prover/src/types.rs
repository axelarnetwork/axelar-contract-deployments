use crate::encoding::{Data, Encoder};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{from_json, HexBinary, StdResult};
use cw_storage_plus::{Key, KeyDeserialize, PrimaryKey};
use sha3::{Digest, Keccak256};
use std::fmt::Display;

#[cw_serde]
pub enum CommandType {
    ApproveContractCall,
    TransferOperatorship,
}

impl Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::ApproveContractCall => write!(f, "approveContractCall"),
            CommandType::TransferOperatorship => write!(f, "transferOperatorship"),
        }
    }
}

#[cw_serde]
pub struct Command {
    pub id: HexBinary,
    pub ty: CommandType,
    pub params: HexBinary,
}

#[cw_serde]
pub struct BatchID(HexBinary);

impl BatchID {
    pub fn new(message_ids: &[String]) -> BatchID {
        let mut message_ids = message_ids.to_vec();
        message_ids.sort();
        // TODO: Must also consider an optional new worker set message.
        Keccak256::digest(message_ids.join(",")).as_slice().into()
    }

    pub fn inner(&self) -> &HexBinary {
        &self.0
    }
}

impl<'a> PrimaryKey<'a> for BatchID {
    type Prefix = ();
    type SubPrefix = ();
    type Suffix = BatchID;
    type SuperSuffix = BatchID;

    fn key(&self) -> Vec<Key> {
        vec![Key::Ref(self.0.as_slice())]
    }
}

impl KeyDeserialize for BatchID {
    type Output = BatchID;

    fn from_vec(value: Vec<u8>) -> StdResult<Self::Output> {
        Ok(from_json(value).expect("violated invariant: BatchID is not deserializable"))
    }
}

impl From<&[u8]> for BatchID {
    fn from(id: &[u8]) -> Self {
        Self(id.into())
    }
}

#[cw_serde]
pub struct CommandBatch {
    pub id: BatchID,
    pub message_ids: Vec<String>,
    pub data: Data,
    pub encoder: Encoder,
}
